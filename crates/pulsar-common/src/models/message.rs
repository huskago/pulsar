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
    pub url: String,
}
