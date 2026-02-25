use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Snowflake,
}
