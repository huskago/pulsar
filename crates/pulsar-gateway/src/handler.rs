use crate::connection::ConnectionManager;
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
use pulsar_messaging::nats_client::NatsClient;
use pulsar_messaging::subjects;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info, warn};

// GET /gateway
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: GatewayState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Wait for the Identify (auth) message
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

    // 2. Register the connection and obtain the receiver
    let mut rx = state.connections.add(user_id.clone()).await;

    // 3. Send the Hello (confirm the connection)
    let hello = ServerEvent::Hello {
        heartbeat_interval: 45000,
    };
    let hello_json = serde_json::to_string(&hello).unwrap();
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

    // NATS chat task -> ConnectionManager
    let chat_connections = state.connections.clone();
    let chat_user_id = user_id.clone();
    let nats_chat_task = tokio::spawn(async move {
        let mut sub = nats_chat_sub;
        while let Some(msg) = sub.next().await {
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            chat_connections.send_to_user(&chat_user_id, event).await;
        }
    });

    // NATS typing task -> ConnectionManager
    let typing_connections = state.connections.clone();
    let typing_user_id = user_id.clone();
    let nats_typing_task = tokio::spawn(async move {
        let mut sub = nats_typing_sub;
        while let Some(msg) = sub.next().await {
            let event: ServerEvent = match serde_json::from_slice(&msg.payload) {
                Ok(e) => e,
                Err(_) => continue,
            };
            typing_connections
                .send_to_user(&typing_user_id, event)
                .await;
        }
    });

    // Send task: rx -> WebSocket
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

    // Receive task: WebSocket -> NATS
    let recv_nats = state.nats.clone();
    let recv_connections = state.connections.clone();
    let recv_db = state.db.clone();
    let recv_user_id = user_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Text(text) => {
                    handle_client_message(
                        &text,
                        &recv_user_id,
                        &recv_nats,
                        &recv_connections,
                        &recv_db,
                    )
                    .await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = nats_chat_task => {},
        _ = nats_typing_task => {},
    }

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
    nats: &NatsClient,
    connections: &ConnectionManager,
    db: &PgPool,
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
            connections
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

            // Vérifier la permission SEND_MESSAGES
            let channel = match pulsar_db::repo::channels::find_by_id(db, ch_id).await {
                Ok(Some(ch)) => ch,
                _ => return,
            };

            let guild = match pulsar_db::repo::guilds::find_by_id(db, channel.guild_id).await {
                Ok(Some(g)) => g,
                _ => return,
            };

            if guild.owner_id != u_id {
                let perms_bits = match pulsar_db::repo::roles::get_member_permissions(
                    db, channel.guild_id, u_id,
                ).await {
                    Ok(p) => p,
                    Err(_) => return,
                };

                let perms = pulsar_common::permissions::Permissions::new(perms_bits);

                if !perms.has(pulsar_common::permissions::Permissions::SEND_MESSAGES) {
                    warn!(user_id = %user_id, channel_id = %channel_id, "No SEND_MESSAGES permission");
                    return;
                }
            }

            let msg_id = chrono::Utc::now().timestamp_millis();

            if let Err(e) =
                pulsar_db::repo::messages::insert(db, msg_id, ch_id, u_id, &content).await
            {
                error!("Failed to persist message: {}", e);
                return;
            }

            let mut attachment_payloads = Vec::new();
            for att in &attachments {
                let att_id = chrono::Utc::now().timestamp_nanos_opt().unwrap();

                match pulsar_db::repo::attachments::insert(
                    db,
                    att_id,
                    msg_id,
                    &att.filename,
                    &att.content_type,
                    att.size,
                    &att.url,
                    &att.url,
                )
                    .await
                {
                    Ok(_) => {
                        info!(att_id = %att_id, msg_id = %msg_id, "Attachment saved to DB");
                        attachment_payloads.push(att.clone());
                    }
                    Err(e) => {
                        error!(att_id = %att_id, msg_id = %msg_id, error = %e, "Failed to insert attachment");
                        attachment_payloads.push(att.clone());
                    }
                }
            }

            let message = pulsar_common::models::message::Message {
                id: Snowflake(msg_id),
                channel_id: Snowflake(ch_id),
                author_id: Snowflake(u_id),
                content,
                attachments: attachment_payloads,
                timestamp: chrono::Utc::now().timestamp_millis(),
                edited_timestamp: None,
            };

            let event = ServerEvent::MessageCreate(message);
            let payload = serde_json::to_vec(&event).unwrap();

            let subject = subjects::chat_channel("default", &channel_id);

            if let Err(e) = nats.publish_persistent(&subject, &payload).await {
                error!("Failed to publish to NATS: {}", e);
            }
        }
        ClientEvent::StartTyping { channel_id } => {
            let event = ServerEvent::TypingStart {
                channel_id: channel_id.clone(),
                user_id: user_id.to_string(),
            };
            let payload = serde_json::to_vec(&event).unwrap();

            let subject = subjects::typing_channel("default", &channel_id);
            let _ = nats.publish(&subject, &payload).await;
        }
        ClientEvent::Identify { .. } => {
            warn!("Received Identify after already authenticated");
        }
    }
}
