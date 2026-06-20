use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub author_id: Snowflake,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentPayload>,
    pub timestamp: i64,
    pub edited_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentPayload {
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    /// MinIO storage key (e.g. "channel_id/uuid.jpg"), used server-side
    #[serde(default)]
    pub key: String,
    /// Presigned URL (15 min), generated on-demand server-side, empty from client
    #[serde(default)]
    pub url: String,
}
