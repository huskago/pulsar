use crate::{middleware::auth::AuthUser, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use pulsar_common::error::AppError;
use pulsar_common::utils::generate_invite_code;
use pulsar_db::repo::{guilds, invites};
use serde::{Deserialize, Serialize};
use tracing::info;
use pulsar_common::permissions::Permissions;
use crate::handlers::perms;

#[derive(Debug, Deserialize)]
pub struct CreateInvite {
    pub max_uses: Option<i32>,
    pub max_age: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub code: String,
    pub guild_id: String,
    pub guild_name: String,
    pub creator_id: String,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<String>,
}

// POST /guilds/:guild_id/invites
pub async fn create_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(payload): Json<CreateInvite>,
) -> Result<Json<InviteResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::CREATE_INVITES).await?;

    let guild = guilds::find_by_id(&state.db, guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    let code = generate_invite_code(10);

    let expires_at = payload
        .max_age
        .map(|seconds| chrono::Utc::now() + chrono::Duration::seconds(seconds));

    let invite = invites::insert(
        &state.db,
        &code,
        guild_id,
        user_id,
        payload.max_uses,
        expires_at,
    )
    .await?;

    info!(code = %code, guild_id = %guild_id, "Invite created");

    Ok(Json(InviteResponse {
        code: invite.code,
        guild_id: invite.guild_id.to_string(),
        guild_name: guild.name,
        creator_id: invite.creator_id.to_string(),
        max_uses: invite.max_uses,
        uses: invite.uses,
        expires_at: invite.expires_at.map(|t| t.to_rfc3339()),
    }))
}

// GET /guilds/:guild_id/invites
pub async fn list_invites(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<InviteResponse>>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }

    let guild = guilds::find_by_id(&state.db, guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    let rows = invites::find_by_guild(&state.db, guild_id).await?;

    let response: Vec<InviteResponse> = rows
        .into_iter()
        .map(|i| InviteResponse {
            code: i.code,
            guild_id: i.guild_id.to_string(),
            guild_name: guild.name.clone(),
            creator_id: i.creator_id.to_string(),
            max_uses: i.max_uses,
            uses: i.uses,
            expires_at: i.expires_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

// GET /invites/:code
pub async fn get_invite(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<InviteResponse>, AppError> {
    let invite = invites::find_by_code(&state.db, &code)
        .await?
        .ok_or(AppError::NotFound("Invite not found".into()))?;

    if let Some(expires_at) = invite.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(AppError::BadRequest("Invite has expired".into()));
        }
    }

    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(AppError::BadRequest("Invite has reached max uses".into()));
        }
    }

    let guild = guilds::find_by_id(&state.db, invite.guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    Ok(Json(InviteResponse {
        code: invite.code,
        guild_id: invite.guild_id.to_string(),
        guild_name: guild.name,
        creator_id: invite.creator_id.to_string(),
        max_uses: invite.max_uses,
        uses: invite.uses,
        expires_at: invite.expires_at.map(|t| t.to_rfc3339()),
    }))
}

// POST /invites/:code/join
pub async fn join_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<InviteResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let invite = invites::find_by_code(&state.db, &code)
        .await?
        .ok_or(AppError::NotFound("Invite not found or expired".into()))?;

    if let Some(expires_at) = invite.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(AppError::BadRequest("Invite has expired".into()));
        }
    }

    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(AppError::BadRequest("Invite has reached max uses".into()));
        }
    }

    if guilds::is_member(&state.db, invite.guild_id, user_id).await? {
        return Err(AppError::BadRequest(
            "Already a member of this guild".into(),
        ));
    }

    let used = invites::use_invite(&state.db, &code).await?;
    if !used {
        return Err(AppError::BadRequest("Invite is no longer valid".into()));
    }

    guilds::add_member(&state.db, invite.guild_id, user_id).await?;

    let guild = guilds::find_by_id(&state.db, invite.guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    info!(
        code = %code,
        guild_id = %invite.guild_id,
        user_id = %user_id,
        "User joined guild via invite"
    );

    Ok(Json(InviteResponse {
        code: invite.code,
        guild_id: invite.guild_id.to_string(),
        guild_name: guild.name,
        creator_id: invite.creator_id.to_string(),
        max_uses: invite.max_uses,
        uses: invite.uses + 1,
        expires_at: invite.expires_at.map(|t| t.to_rfc3339()),
    }))
}
