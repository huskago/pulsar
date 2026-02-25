use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub author_id: Snowflake,
    pub content: String,
    pub timestamp: i64,
    pub edited_timestamp: Option<i64>,
}
