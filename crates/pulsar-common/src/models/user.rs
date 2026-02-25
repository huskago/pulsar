use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: Option<String>,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    #[default]
    Offline,
    Online,
    Idle,
    DoNotDisturb,
}
