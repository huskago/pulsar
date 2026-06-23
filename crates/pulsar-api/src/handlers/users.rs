use axum::{extract::State, Json};
use pulsar_common::{
    error::AppError,
    models::user::{SelfUserResponse, UserStatus},
};
use pulsar_db::repo::users;
use serde::{Deserialize, Serialize};

use crate::{middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub dm_privacy: String,
    pub friend_request_privacy: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettings {
    pub dm_privacy: Option<String>,
    pub friend_request_privacy: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfile {
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
}

pub async fn update_me(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<SelfUserResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    let current = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let username = payload.username.as_deref().unwrap_or(&current.username);
    if username.len() < 2 || username.len() > 32 {
        return Err(AppError::BadRequest("Username must be between 2 and 32 characters".into()));
    }

    if let Some(ref url) = payload.avatar_url {
        if !url.is_empty() && !url.starts_with(&state.storage_endpoint) {
            return Err(AppError::BadRequest("avatar_url must point to the configured storage endpoint".into()));
        }
    }
    let avatar_url = payload.avatar_url.as_deref().or(current.avatar_url.as_deref());
    let status = payload.status.as_deref().unwrap_or(&current.status);
    let valid_statuses = ["online", "idle", "dnd", "offline"];
    if !valid_statuses.contains(&status) {
        return Err(AppError::BadRequest("Invalid status".into()));
    }

    let row = users::update_profile(&state.db, user_id, username, avatar_url, status).await?;

    Ok(Json(SelfUserResponse {
        id: row.id.to_string(),
        username: row.username,
        email: row.email,
        avatar_url: row.avatar_url,
        status: UserStatus::from_db(&row.status),
    }))
}

pub async fn get_me(auth: AuthUser, State(state): State<AppState>) -> Result<Json<SelfUserResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let row = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(Json(SelfUserResponse {
        id: row.id.to_string(),
        username: row.username,
        email: row.email,
        avatar_url: row.avatar_url,
        status: UserStatus::from_db(&row.status),
    }))
}

pub async fn get_settings(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SettingsResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let settings = users::get_settings(&state.db, user_id).await?;

    Ok(Json(SettingsResponse {
        dm_privacy: settings.dm_privacy,
        friend_request_privacy: settings.friend_request_privacy,
    }))
}

pub async fn update_settings(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettings>,
) -> Result<Json<SettingsResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let current = users::get_settings(&state.db, user_id).await?;

    const VALID_DM_PRIVACY: &[&str] = &[
        "everyone", "friends_and_guilds", "friends_of_friends", "friends_only", "nobody",
    ];
    const VALID_FR_PRIVACY: &[&str] = &[
        "everyone", "friends_of_friends", "guilds_only", "nobody",
    ];

    let dm_priv = match payload.dm_privacy {
        None => current.dm_privacy,
        Some(s) => {
            if !VALID_DM_PRIVACY.contains(&s.as_str()) {
                return Err(AppError::BadRequest(format!("Invalid dm_privacy: {}", s)));
            }
            s
        }
    };

    let fr_priv = match payload.friend_request_privacy {
        None => current.friend_request_privacy,
        Some(s) => {
            if !VALID_FR_PRIVACY.contains(&s.as_str()) {
                return Err(AppError::BadRequest(format!("Invalid friend_request_privacy: {}", s)));
            }
            s
        }
    };

    let updated = users::update_settings(&state.db, user_id, &dm_priv, &fr_priv).await?;

    Ok(Json(SettingsResponse {
        dm_privacy: updated.dm_privacy,
        friend_request_privacy: updated.friend_request_privacy,
    }))
}
