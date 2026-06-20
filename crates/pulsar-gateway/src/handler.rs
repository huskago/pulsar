use crate::state::GatewayState;
use axum::{
    extract::{
        ws::{Message, WebSocket}, State,
        WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use pulsar_common::models::{
    event::{ClientEvent, ServerEvent},
    snowflake::Snowflake,
};
use pulsar_messaging::subjects;
use std::time::Duration;
use tracing::{error, info, warn};

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: GatewayState) {
    let (mut sender, mut receiver) = socket.split();

    let claims = match tokio::time::timeout(
        Duration::from_secs(10),
        wait_for_identity(&mut receiver, &state),
    )
    .await
    {
        Ok(Ok(claims)) => claims,
        Ok(Err(e)) => {
            warn!("Identity failed: {}", e);
            return;
        }
        Err(_) => {
            warn!("Identity timeout");
            return;
        }
    };

    let user_id = claims.sub.clone();
    info!(user_id = %user_id, "Client authenticated on gateway");

    let mut rx = state.connections.add(user_id.clone()).await;

    let hello = ServerEvent::Hello {
        heartbeat_interval: 45000,
    };
    let hello_json = match serde_json::to_string(&hello) {
        Ok(j) => j,
        Err(e) => { error!("Failed to serialize Hello: {}", e); state.connections.remove(&user_id).await; return; }
    };
    if sender.send(Message::Text(hello_json.into())).await.is_err() {
        state.connections.remove(&user_id).await;
        return;
    }

    let nats_chat_sub = match state.nats.subscribe("chat.>").await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to NATS chat: {}", e);
            state.connections.remove(&user_id).await;
            return;
        }
    };

    let nats_typing_sub = match state.nats.subscribe("typing.>").await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to NATS typing: {}", e);
            state.connections.remove(&user_id).await;
            return;
        }
    };

    let nats_dm_sub = match state.nats.subscribe(&"dm.>".to_string()).await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to DM NATS: {}", e);
            state.connections.remove(&user_id).await;
            return;
        }
    };

    // subject: chat.{guild_id}.{channel_id}
    let chat_connections = state.connections.clone();
    let chat_user_id = user_id.clone();
    let chat_db = state.db.clone();
    let chat_uid: i64 = user_id.parse().unwrap_or(0);
    let nats_chat_task = tokio::spawn(async move {
        let mut sub = nats_chat_sub;
        while let Some(msg) = sub.next().await {
            let subject = msg.subject.as_str();
            let guild_id: i64 = match subject.split('.').nth(1).and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => continue,
            };
            let is_member = pulsar_db::repo::guilds::is_member(&chat_db, guild_id, chat_uid)
                .await
                .unwrap_or(false);
            if !is_member {
                continue;
            }
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            chat_connections.send_to_user(&chat_user_id, event).await;
        }
    });

    // subject: typing.{guild_id}.{channel_id}
    let typing_connections = state.connections.clone();
    let typing_user_id = user_id.clone();
    let typing_db = state.db.clone();
    let typing_uid: i64 = user_id.parse().unwrap_or(0);
    let nats_typing_task = tokio::spawn(async move {
        let mut sub = nats_typing_sub;
        while let Some(msg) = sub.next().await {
            let subject = msg.subject.as_str();
            let guild_id: i64 = match subject.split('.').nth(1).and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => continue,
            };
            let is_member = pulsar_db::repo::guilds::is_member(&typing_db, guild_id, typing_uid)
                .await
                .unwrap_or(false);
            if !is_member {
                continue;
            }
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            typing_connections
                .send_to_user(&typing_user_id, event)
                .await;
        }
    });

    let dm_connections = state.connections.clone();
    let dm_user_id = user_id.clone();
    let dm_db = state.db.clone();
    let nats_dm_task = tokio::spawn(async move {
        let mut sub = nats_dm_sub;
        while let Some(msg) = sub.next().await {
            let subject = msg.subject.as_str();
            let dm_channel_id: i64 = match subject.strip_prefix("dm.").and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => continue,
            };

            let is_participant = pulsar_db::repo::dms::is_participant(
                &dm_db, dm_channel_id, dm_user_id.parse().unwrap_or(0)
            ).await.unwrap_or(false);

            if !is_participant {
                continue;
            }

            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            dm_connections.send_to_user(&dm_user_id, event).await;
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let json = match serde_json::to_string(&event) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_state = state.clone();
    let recv_user_id = user_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Text(text) => {
                    handle_client_message(
                        &text,
                        &recv_user_id,
                        &recv_state,
                    )
                    .await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let send_abort = send_task.abort_handle();
    let recv_abort = recv_task.abort_handle();
    let chat_abort = nats_chat_task.abort_handle();
    let typing_abort = nats_typing_task.abort_handle();
    let dm_abort = nats_dm_task.abort_handle();

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = nats_chat_task => {},
        _ = nats_typing_task => {},
        _ = nats_dm_task => {},
    }

    // Dropping a JoinHandle only detaches the task, abort to actually stop it.
    send_abort.abort();
    recv_abort.abort();
    chat_abort.abort();
    typing_abort.abort();
    dm_abort.abort();

    state.connections.remove(&user_id).await;
    info!(user_id = %user_id, "Client disconnected from gateway");
}

async fn wait_for_identity(
    reader: &mut (impl StreamExt<Item = Result<Message, axum::Error>> + Unpin),
    state: &GatewayState,
) -> Result<pulsar_auth::jwt::Claims, String> {
    while let Some(Ok(msg)) = reader.next().await {
        if let Message::Text(text) = msg {
            let event: ClientEvent =
                serde_json::from_str(&text).map_err(|e| format!("Invalid JSON: {}", e))?;

            match event {
                ClientEvent::Identify { token } => {
                    let claims = state
                        .jwt
                        .validate_token(&token)
                        .map_err(|_| "Invalid token".to_string())?;
                    return Ok(claims);
                }
                _ => return Err("Expected Identify as first message".into()),
            }
        }
    }

    Err("Connection closed before Identify".into())
}

async fn handle_client_message(
    text: &str,
    user_id: &str,
    state: &GatewayState,
) {
    let event: ClientEvent = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            warn!("Invalid client event: {}", e);
            return;
        }
    };

    match event {
        ClientEvent::Heartbeat => {
            state.connections
                .send_to_user(user_id, ServerEvent::HeartbeatAck)
                .await;
        }
        ClientEvent::SendMessage {
            channel_id,
            content,
            attachments,
        } => {
            let ch_id: i64 = match channel_id.parse() {
                Ok(id) => id,
                Err(_) => return,
            };
            let u_id: i64 = match user_id.parse() {
                Ok(id) => id,
                Err(_) => return,
            };

            let channel = match pulsar_db::repo::channels::find_by_id(&state.db, ch_id).await {
                Ok(Some(ch)) => ch,
                _ => return,
            };

            if channel.kind == "dm" {
                match pulsar_db::repo::dms::is_participant(&state.db, ch_id, u_id).await {
                    Ok(true) => {},
                    _ => {
                        warn!(user_id = %user_id, "Not a DM participant");
                        return;
                    }
                }
            } else if let Some(gid) = channel.guild_id {
                let guild = match pulsar_db::repo::guilds::find_by_id(&state.db, gid).await {
                    Ok(Some(g)) => g,
                    _ => return,
                };

                if guild.owner_id != u_id {
                    let perms_bits = match pulsar_db::repo::roles::get_member_permissions(&state.db, gid, u_id).await {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    let perms = pulsar_common::permissions::Permissions::new(perms_bits);
                    if !perms.has(pulsar_common::permissions::Permissions::SEND_MESSAGES) {
                        warn!(user_id = %user_id, "No SEND_MESSAGES permission");
                        return;
                    }
                }
            } else {
                return;
            }

            // Retrieve the channel DEK
            let dek = match crate::crypto_helpers::get_channel_dek(state, ch_id).await {
                Ok(d) => d,
                Err(e) => {
                    error!("Cannot get DEK for channel {}: {}", ch_id, e);
                    return;
                }
            };

            // Encrypt the message content before storing
            let encrypted_content = match state.crypto.encrypt_message(&dek, &content) {
                Ok(ct) => ct,
                Err(e) => {
                    error!("Encryption failed: {}", e);
                    return;
                }
            };

            let msg_id = pulsar_common::models::snowflake::Snowflake::generate().0;

            // Persist encrypted content to ScyllaDB (replaces pulsar_db::repo::messages::insert)
            let scylla_msg = pulsar_scylla::messages::ScyllaMessage {
                channel_id: ch_id,
                message_id: msg_id,
                author_id: u_id,
                content: encrypted_content,
                edited_at: None,
            };
            if let Err(e) = pulsar_scylla::messages::insert(&state.scylla, &scylla_msg).await {
                error!("Failed to persist message to ScyllaDB: {}", e);
                return;
            }

            // Build attachment list with presigned URLs
            let mut attachment_payloads = Vec::new();
            for att in &attachments {
                let url = if !att.key.is_empty() {
                    state.storage.generate_presigned_url(&att.key, 900)
                        .await
                        .unwrap_or_default()
                } else {
                    att.url.clone()
                };

                let att_id = pulsar_common::models::snowflake::Snowflake::generate().0;
                if let Err(e) = pulsar_db::repo::attachments::insert(
                    &state.db,
                    att_id,
                    msg_id,
                    &att.filename,
                    &att.content_type,
                    att.size,
                    &att.key,
                    &url,
                ).await {
                    error!(att_id = %att_id, msg_id = %msg_id, error = %e, "Failed to insert attachment, skipping from payload");
                    continue;
                }

                info!(att_id = %att_id, msg_id = %msg_id, "Attachment saved to DB");
                attachment_payloads.push(pulsar_common::models::message::AttachmentPayload {
                    filename: att.filename.clone(),
                    content_type: att.content_type.clone(),
                    size: att.size,
                    key: att.key.clone(),
                    url,
                });
            }

            // Broadcast plaintext content to connected clients via NATS
            let message = pulsar_common::models::message::Message {
                id: Snowflake(msg_id),
                channel_id: Snowflake(ch_id),
                author_id: Snowflake(u_id),
                content,  // plaintext — encryption is at-rest in ScyllaDB only
                attachments: attachment_payloads,
                timestamp: chrono::Utc::now().timestamp_millis(),
                edited_timestamp: None,
            };

            let event = ServerEvent::MessageCreate(message);
            let payload = match serde_json::to_vec(&event) {
                Ok(p) => p,
                Err(e) => { error!("Failed to serialize MessageCreate: {}", e); return; }
            };

            let subject = if channel.kind == "dm" {
                format!("dm.{}", channel_id)
            } else {
                subjects::chat_channel("default", &channel_id)
            };

            if let Err(e) = state.nats.publish_persistent(&subject, &payload).await {
                error!("Failed to publish to NATS: {}", e);
            }
        }
        ClientEvent::StartTyping { channel_id } => {
            let event = ServerEvent::TypingStart {
                channel_id: channel_id.clone(),
                user_id: user_id.to_string(),
            };
            let payload = match serde_json::to_vec(&event) {
                Ok(p) => p,
                Err(e) => { error!("Failed to serialize TypingStart: {}", e); return; }
            };

            let subject = subjects::typing_channel("default", &channel_id);
            let _ = state.nats.publish(&subject, &payload).await;
        }
        ClientEvent::Identify { .. } => {
            warn!("Received Identify after already authenticated");
        }
    }
}
