use axum::{extract::{Path, State}, Json};
use pulsar_common::error::AppError;
use pulsar_common::permissions::Permissions;
use pulsar_db::repo::{attachments, channel_keys, channels as channels_repo, guilds, roles};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Deserialize)]
pub struct CreateGuild {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct GuildResponse {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: String,
}

pub async fn create_guild(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateGuild>,
) -> Result<Json<GuildResponse>, AppError> {
    if payload.name.len() < 2 || payload.name.len() > 100 {
        return Err(AppError::BadRequest(
            "Guild name must be between 2 and 100 characters".into(),
        ));
    }

    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let owned = guilds::count_owned_by_user(&state.db, user_id).await?;
    if owned >= 100 {
        return Err(AppError::BadRequest("Guild limit reached (max 100 owned guilds)".into()));
    }

    let guild_id = pulsar_common::models::snowflake::Snowflake::generate().0;

    let guild = guilds::insert(&state.db, guild_id, &payload.name, user_id).await?;

    let role_id = pulsar_common::models::snowflake::Snowflake::generate().0;
    roles::create_default_role(&state.db, role_id, guild_id, Permissions::DEFAULT).await?;

    let channel_id = pulsar_common::models::snowflake::Snowflake::generate().0;
    channels_repo::insert(&state.db, channel_id, guild_id, "general", "text", 0).await?;

    let dek = state.crypto.generate_dek();
    let sealed = state.crypto.seal_dek(&dek)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DEK seal failed: {}", e)))?;
    channel_keys::upsert(&state.db, channel_id, &sealed).await?;

    info!(guild_id = %guild.id, "Guild created with @everyone role");

    Ok(Json(GuildResponse {
        id: guild.id.to_string(),
        name: guild.name,
        icon_url: guild.icon_url,
        owner_id: guild.owner_id.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuild {
    pub name: Option<String>,
    pub icon_url: Option<String>,
}

pub async fn update_guild(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(payload): Json<UpdateGuild>,
) -> Result<Json<GuildResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let guild = guilds::find_by_id(&state.db, guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    if guild.owner_id != user_id {
        return Err(AppError::Forbidden);
    }

    let name = payload.name.as_deref().unwrap_or(&guild.name);
    if name.len() < 2 || name.len() > 100 {
        return Err(AppError::BadRequest("Guild name must be between 2 and 100 characters".into()));
    }
    if let Some(ref url) = payload.icon_url {
        if !url.is_empty() && !url.starts_with(&state.storage_endpoint) {
            return Err(AppError::BadRequest("icon_url must point to the configured storage endpoint".into()));
        }
    }
    let icon_url = payload.icon_url.as_deref().or(guild.icon_url.as_deref());

    let updated = guilds::update(&state.db, guild_id, name, icon_url).await?;

    Ok(Json(GuildResponse {
        id: updated.id.to_string(),
        name: updated.name,
        icon_url: updated.icon_url,
        owner_id: updated.owner_id.to_string(),
    }))
}

pub async fn delete_guild(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<axum::http::StatusCode, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    let guild = guilds::find_by_id(&state.db, guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    if guild.owner_id != user_id {
        return Err(AppError::Forbidden);
    }

    let channels = channels_repo::find_by_guild(&state.db, guild_id).await.unwrap_or_default();
    for ch in &channels {
        let scylla_msgs = pulsar_scylla::messages::find_by_channel(
            &state.scylla, ch.id, 10_000i32, None,
        ).await.unwrap_or_default();

        let message_ids: Vec<i64> = scylla_msgs.iter().map(|m| m.message_id).collect();
        if !message_ids.is_empty() {
            let atts = attachments::find_by_messages(&state.db, &message_ids).await.unwrap_or_default();
            for att in &atts {
                if let Err(e) = state.storage.delete(&att.storage_key).await {
                    tracing::warn!("Failed to delete attachment {} from storage: {}", att.storage_key, e);
                }
            }
            let _ = attachments::delete_by_message_ids(&state.db, &message_ids).await;
        }

        if let Err(e) = pulsar_scylla::messages::delete_by_channel(&state.scylla, ch.id).await {
            tracing::warn!("Failed to delete ScyllaDB messages for channel {}: {}", ch.id, e);
        }
    }

    guilds::delete(&state.db, guild_id).await?;
    info!(guild_id = %guild_id, "Guild deleted");

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_guilds(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<GuildResponse>>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let rows = guilds::find_by_user(&state.db, user_id).await?;

    let response: Vec<GuildResponse> = rows
        .into_iter()
        .map(|g| GuildResponse {
            id: g.id.to_string(),
            name: g.name,
            icon_url: g.icon_url,
            owner_id: g.owner_id.to_string(),
        })
        .collect();

    Ok(Json(response))
}
