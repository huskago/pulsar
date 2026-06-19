use axum::{extract::State, Json};
use pulsar_common::error::AppError;
use pulsar_common::permissions::Permissions;
use pulsar_db::repo::{channels as channels_repo, guilds, roles};
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

// POST /guilds
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
    let guild_id = pulsar_common::models::snowflake::Snowflake::generate().0;

    let guild = guilds::insert(&state.db, guild_id, &payload.name, user_id).await?;

    let role_id = pulsar_common::models::snowflake::Snowflake::generate().0;
    roles::create_default_role(&state.db, role_id, guild_id, Permissions::DEFAULT).await?;

    let channel_id = pulsar_common::models::snowflake::Snowflake::generate().0;
    channels_repo::insert(&state.db, channel_id, guild_id, "general", "text", 0).await?;

    info!(guild_id = %guild.id, "Guild created with @everyone role");

    Ok(Json(GuildResponse {
        id: guild.id.to_string(),
        name: guild.name,
        icon_url: guild.icon_url,
        owner_id: guild.owner_id.to_string(),
    }))
}

// GET /guilds
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
