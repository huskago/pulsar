use axum::{
    extract::{Path, State},
    Json,
};
use pulsar_common::error::AppError;
use pulsar_db::repo::{channels, guilds, messages};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::{middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub name: String,
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    kind: String,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    pub timestamp: i64,
    pub edited_timestamp: Option<i64>,
}

// POST /guilds/:guild_id/channels
pub async fn create_channel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(payload): Json<CreateChannel>,
) -> Result<Json<ChannelResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }

    if payload.name.len() < 1 || payload.name.len() > 100 {
        return Err(AppError::BadRequest(
            "Channel name must be between 1 and 100 characters".into(),
        ));
    }

    let kind = payload.kind.as_deref().unwrap_or("text");
    if !["text", "voice", "category"].contains(&kind) {
        return Err(AppError::BadRequest("Invalid channel kind".into()));
    }

    let channel_id = chrono::Utc::now().timestamp_millis();

    let existing = channels::find_by_guild(&state.db, guild_id).await?;
    let position = existing.len() as i32;

    let row = channels::insert(&state.db, channel_id, guild_id, &payload.name, kind, position).await?;

    info!(channel_id = %row.id, guild_id = %guild_id, "Channel created");

    Ok(Json(ChannelResponse {
        id: row.id.to_string(),
        guild_id: row.guild_id.to_string(),
        name: row.name,
        kind: row.kind,
        position: row.position,
    }))
}

// GET /guilds/:guild_id/channels
pub async fn list_channels(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<ChannelResponse>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }

    let rows = channels::find_by_guild(&state.db, guild_id).await?;

    let response: Vec<ChannelResponse> = rows
        .into_iter()
        .map(|c| ChannelResponse {
            id: c.id.to_string(),
            guild_id: c.guild_id.to_string(),
            name: c.name,
            kind: c.kind,
            position: c.position,
        })
        .collect();

    Ok(Json(response))
}

// GET /channels/:channel_id/messages?limit=50&before=snowflake_id
pub async fn list_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, AppError> {
    let _user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let channel_id: i64 = channel_id.parse().map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    let limit = params.limit.unwrap_or(50).min(100);
    let before = params.before.and_then(|b| b.parse::<i64>().ok());

    let rows = messages::find_by_channel(&state.db, channel_id, limit, before).await?;

    let response: Vec<MessageResponse> = rows
        .into_iter()
        .rev()
        .map(|m| MessageResponse {
            id: m.id.to_string(),
            channel_id: m.channel_id.to_string(),
            author_id: m.author_id.to_string(),
            content: m.content,
            timestamp: m.created_at.timestamp_millis(),
            edited_timestamp: m.edited_at.map(|t| t.timestamp_millis()),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub limit: Option<i64>,
    pub before: Option<String>,
}