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

    // 4. Two parallel tasks
    // - One that reads messages from the client (receiver → processing)
    // - One that sends events to the client (rx → sender)

    // Send task: rx → WebSocket
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

    // Receive task: WebSocket → processing
    let connections = state.connections.clone();
    let recv_user_id = user_id.clone();

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_client_message(&text, &recv_user_id, &connections).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
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

async fn handle_client_message(text: &str, user_id: &str, connections: &ConnectionManager) {
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
        } => {
            let message = pulsar_common::models::message::Message {
                id: Snowflake(chrono::Utc::now().timestamp_millis()),
                channel_id: Snowflake(channel_id.parse().unwrap_or(0)),
                author_id: Snowflake(user_id.parse().unwrap_or(0)),
                content,
                timestamp: chrono::Utc::now().timestamp_millis(),
                edited_timestamp: None,
            };

            // TODO: NATS
            connections
                .broadcast(ServerEvent::MessageCreate(message))
                .await;
        }
        ClientEvent::StartTyping { channel_id } => {
            connections
                .broadcast(ServerEvent::TypingStart {
                    channel_id,
                    user_id: user_id.to_string(),
                })
                .await;
        }
        ClientEvent::Identify { .. } => {
            warn!("Received Identify after already authenticated");
        }
    }
}
