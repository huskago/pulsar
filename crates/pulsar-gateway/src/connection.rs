use pulsar_common::models::event::ServerEvent;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    pub sender: mpsc::UnboundedSender<ServerEvent>,
}

#[derive(Clone, Default)]
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, Vec<ConnectionHandle>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add(&self, user_id: String) -> mpsc::UnboundedReceiver<ServerEvent> {
        let (tx, rx) = mpsc::unbounded_channel();

        let handle = ConnectionHandle {
            sender: tx,
        };

        self.connections
            .write()
            .await
            .entry(user_id.clone())
            .or_default()
            .push(handle);

        info!(user_id = %user_id, "Connection added");
        rx
    }

    pub async fn remove(&self, user_id: &str) {
        let mut conns = self.connections.write().await;

        if let Some(handles) = conns.get_mut(user_id) {
            handles.retain(|h| !h.sender.is_closed());

            if handles.is_empty() {
                conns.remove(user_id);
                info!(user_id = %user_id, "All connections removed");
            }
        }
    }

    pub async fn send_to_user(&self, user_id: &str, event: ServerEvent) {
        let conns = self.connections.read().await;

        if let Some(handles) = conns.get(user_id) {
            for handle in handles {
                if handle.sender.send(event.clone()).is_err() {
                    warn!(
                        user_id = %user_id,
                        "Failed to send to connection (likely closed)",
                    );
                }
            }
        }
    }
}
