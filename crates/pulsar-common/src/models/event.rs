use super::{message::Message, user::UserStatus};
use crate::models::message::AttachmentPayload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    Hello { heartbeat_interval: u64 },
    MessageCreate(Message),
    PresenceUpdate { user_id: String, status: UserStatus },
    TypingStart { channel_id: String, user_id: String },
    HeartbeatAck,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    Identify {
        token: String,
    },
    Heartbeat,
    SendMessage {
        channel_id: String,
        content: String,
        #[serde(default)]
        attachments: Vec<AttachmentPayload>,
    },
    StartTyping {
        channel_id: String,
    },
}
