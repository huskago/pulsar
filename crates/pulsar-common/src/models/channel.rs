use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub kind: ChannelKind,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Text,
    Voice,
    Category,
}
