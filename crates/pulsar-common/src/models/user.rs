use super::snowflake::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
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
    #[serde(rename = "dnd")]
    DoNotDisturb,
}

impl UserStatus {
    pub fn from_db(s: &str) -> Self {
        match s {
            "online" => Self::Online,
            "idle" => Self::Idle,
            "dnd" => Self::DoNotDisturb,
            _ => Self::Offline,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelfUserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUserResponse {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: SelfUserResponse,
}
