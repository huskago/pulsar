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
        Err(e) => { error!("Failed to serialize Hello: {}", e); drop(rx); state.connections.remove(&user_id).await; return; }
    };
    if sender.send(Message::Text(hello_json.into())).await.is_err() {
        drop(rx);
        state.connections.remove(&user_id).await;
        return;
    }

    let uid: i64 = match user_id.parse() {
        Ok(v) => v,
        Err(_) => { state.connections.remove(&user_id).await; return; }
    };

    let user_guilds = match pulsar_db::repo::guilds::list_for_user(&state.db, uid).await {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to fetch guilds for user {}: {}", uid, e);
            state.connections.remove(&user_id).await;
            return;
        }
    };
    let guild_ids: Vec<i64> = user_guilds.iter().map(|g| g.id).collect();

    let mut guild_subs: Vec<async_nats::Subscriber> = Vec::new();
    for &gid in &guild_ids {
        if let Ok(sub) = state.nats.subscribe(&subjects::chat_guild(gid)).await {
            guild_subs.push(sub);
        }
        if let Ok(sub) = state.nats.subscribe(&subjects::typing_guild(gid)).await {
            guild_subs.push(sub);
        }
        if let Ok(sub) = state.nats.subscribe(&subjects::presence_guild(gid)).await {
            guild_subs.push(sub);
        }
    }

    // Subscribe to personal DM inbox
    let dm_subject = subjects::dm_user(uid);
    let nats_dm_sub = match state.nats.subscribe(&dm_subject).await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to DM inbox: {}", e);
            state.connections.remove(&user_id).await;
            return;
        }
    };

    let guild_connections = state.connections.clone();
    let guild_user_id = user_id.clone();
    let nats_guild_task = tokio::spawn(async move {
        use futures::StreamExt;
        let mut combined = futures::stream::select_all(guild_subs);
        while let Some(msg) = combined.next().await {
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            guild_connections.send_to_user(&guild_user_id, event).await;
        }
    });

    let dm_connections = state.connections.clone();
    let dm_user_id = user_id.clone();
    let nats_dm_task = tokio::spawn(async move {
        let mut sub = nats_dm_sub;
        while let Some(msg) = sub.next().await {
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            dm_connections.send_to_user(&dm_user_id, event).await;
        }
    });

    {
        let db = state.db.clone();
        let nats = state.nats.clone();
        let gids = guild_ids.clone();
        let uid_str = user_id.clone();
        tokio::spawn(async move {
            let _ = pulsar_db::repo::users::update_status(&db, uid, "online").await;
            let event = ServerEvent::PresenceUpdate {
                user_id: uid_str,
                status: pulsar_common::models::user::UserStatus::Online,
            };
            if let Ok(payload) = serde_json::to_vec(&event) {
                for &gid in &gids {
                    let _ = nats.publish(&subjects::presence_guild(gid), &payload).await;
                }
            }
        });
    }

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
        let heartbeat_timeout = Duration::from_secs(65);
        loop {
            match tokio::time::timeout(heartbeat_timeout, receiver.next()).await {
                Ok(Some(Ok(message))) => match message {
                    Message::Text(text) => {
                        handle_client_message(&text, &recv_user_id, &recv_state).await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                },
                Ok(Some(Err(_))) | Ok(None) => break,
                Err(_) => {
                    warn!(user_id = %recv_user_id, "Heartbeat timeout, closing connection");
                    break;
                }
            }
        }
    });

    let guild_abort = nats_guild_task.abort_handle();
    let dm_abort = nats_dm_task.abort_handle();

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    guild_abort.abort();
    dm_abort.abort();

    state.connections.remove(&user_id).await;
    info!(user_id = %user_id, "Client disconnected from gateway");

    {
        let db = state.db.clone();
        let nats = state.nats.clone();
        let uid_str = user_id.clone();
        tokio::spawn(async move {
            let _ = pulsar_db::repo::users::update_status(&db, uid, "offline").await;
            let event = ServerEvent::PresenceUpdate {
                user_id: uid_str,
                status: pulsar_common::models::user::UserStatus::Offline,
            };
            if let Ok(payload) = serde_json::to_vec(&event) {
                for &gid in &guild_ids {
                    let _ = nats.publish(&subjects::presence_guild(gid), &payload).await;
                }
            }
        });
    }
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
                    match crate::redis_client::is_jwt_blocked(&state.redis, &claims.jti).await {
                        Ok(true) => return Err("Token has been revoked".to_string()),
                        Ok(false) => {}
                        Err(e) => {
                            warn!("Redis blocklist check failed: {}", e);
                            return Err("Authentication error".to_string());
                        }
                    }
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
            if !state.rate_limiter.check(user_id).await {
                warn!(user_id = %user_id, "SendMessage rate limit exceeded");
                return;
            }
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

            if content.len() > 4000 {
                warn!(user_id = %user_id, "SendMessage content exceeds 4000 chars, rejected");
                return;
            }

            let dek = match crate::crypto_helpers::get_channel_dek(state, ch_id).await {
                Ok(d) => d,
                Err(e) => {
                    error!("Cannot get DEK for channel {}: {}", ch_id, e);
                    return;
                }
            };

            let encrypted_content = match state.crypto.encrypt_message(&dek, &content) {
                Ok(ct) => ct,
                Err(e) => {
                    error!("Encryption failed: {}", e);
                    return;
                }
            };

            let msg_id = pulsar_common::models::snowflake::Snowflake::generate().0;

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

            let mut attachment_payloads = Vec::new();
            for att in &attachments {
                if att.key.is_empty() {
                    warn!(user_id = %user_id, "Rejected attachment without server key");
                    continue;
                }
                match pulsar_db::repo::pending_attachments::take(&state.db, u_id, &att.key).await {
                    Ok(true) => {}
                    Ok(false) => {
                        warn!(user_id = %user_id, key = %att.key, "Rejected attachment with unrecognised key");
                        continue;
                    }
                    Err(e) => {
                        error!("pending_attachments lookup failed: {}", e);
                        continue;
                    }
                }
                let url = state.storage.generate_presigned_url(&att.key, 900)
                    .await
                    .unwrap_or_default();

                let att_id = pulsar_common::models::snowflake::Snowflake::generate().0;
                if let Err(e) = pulsar_db::repo::attachments::insert(
                    &state.db,
                    pulsar_db::repo::attachments::NewAttachment {
                        id: att_id,
                        message_id: msg_id,
                        filename: &att.filename,
                        content_type: &att.content_type,
                        size: att.size,
                        storage_key: &att.key,
                        url: &url,
                    },
                ).await {
                    error!(att_id = %att_id, msg_id = %msg_id, error = %e, "Failed to insert attachment, aborting send");
                    return;
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

            let message = pulsar_common::models::message::Message {
                id: Snowflake(msg_id),
                channel_id: Snowflake(ch_id),
                author_id: Snowflake(u_id),
                content, // plaintext — clients receive cleartext; encryption is at-rest in ScyllaDB
                attachments: attachment_payloads,
                timestamp: chrono::Utc::now().timestamp_millis(),
                edited_timestamp: None,
            };

            let event = ServerEvent::MessageCreate(message);
            let payload = match serde_json::to_vec(&event) {
                Ok(p) => p,
                Err(e) => { error!("Failed to serialize MessageCreate: {}", e); return; }
            };

            if channel.kind == "dm" {
                if let Err(e) = pulsar_db::repo::dms::update_last_message_at(&state.db, ch_id).await {
                    error!("Failed to update last_message_at for DM {}: {}", ch_id, e);
                }
                match pulsar_db::repo::dms::get_participants(&state.db, ch_id).await {
                    Ok(participants) => {
                        for participant_id in participants {
                            let subject = subjects::dm_user(participant_id);
                            if let Err(e) = state.nats.publish(&subject, &payload).await {
                                error!("Failed to publish DM to user {}: {}", participant_id, e);
                            }
                        }
                    }
                    Err(e) => error!("Failed to get DM participants for channel {}: {}", ch_id, e),
                }
            } else if let Some(gid) = channel.guild_id {
                let subject = subjects::chat_channel(gid, ch_id);
                if let Err(e) = state.nats.publish_persistent(&subject, &payload).await {
                    error!("Failed to publish to NATS: {}", e);
                }
            }
        }
        ClientEvent::StartTyping { channel_id } => {
            if !state.rate_limiter.check(user_id).await {
                warn!(user_id = %user_id, "StartTyping rate limit exceeded");
                return;
            }

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
                    Ok(true) => {}
                    _ => return,
                }
            } else if let Some(gid) = channel.guild_id {
                match pulsar_db::repo::guilds::is_member(&state.db, gid, u_id).await {
                    Ok(true) => {}
                    _ => return,
                }
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
                        return;
                    }
                }
            } else {
                return;
            }

            let event = ServerEvent::TypingStart {
                channel_id: channel_id.clone(),
                user_id: user_id.to_string(),
            };
            let payload = match serde_json::to_vec(&event) {
                Ok(p) => p,
                Err(e) => { error!("Failed to serialize TypingStart: {}", e); return; }
            };

            if channel.kind == "dm" {
                match pulsar_db::repo::dms::get_participants(&state.db, ch_id).await {
                    Ok(participants) => {
                        for participant_id in participants {
                            let subject = subjects::dm_user(participant_id);
                            let _ = state.nats.publish(&subject, &payload).await;
                        }
                    }
                    Err(e) => error!("Failed to get DM participants: {}", e),
                }
            } else if let Some(gid) = channel.guild_id {
                let subject = subjects::typing_channel(gid, ch_id);
                let _ = state.nats.publish(&subject, &payload).await;
            }
        }
        ClientEvent::Identify { .. } => {
            warn!("Received Identify after already authenticated");
        }
    }
}
