use axum::{
    extract::{Path, State},
    Json,
};
use pulsar_common::{error::AppError, permissions::Permissions};
use pulsar_db::repo::{guilds, roles};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{handlers::perms, middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: i64,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateRole {
    pub name: String,
    pub color: Option<i32>,
    pub permissions: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub color: Option<i32>,
    pub permissions: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleBody {
    pub user_id: String,
}

fn to_response(r: roles::RoleRow) -> RoleResponse {
    RoleResponse {
        id: r.id.to_string(),
        guild_id: r.guild_id.to_string(),
        name: r.name,
        color: r.color,
        position: r.position,
        permissions: r.permissions,
        is_default: r.is_default,
    }
}

pub async fn list_roles(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<RoleResponse>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden)
    }

    let rows = roles::find_by_guild(&state.db, guild_id).await?;
    Ok(Json(rows.into_iter().map(to_response).collect()))
}

pub async fn create_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(payload): Json<CreateRole>,
) -> Result<Json<RoleResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::MANAGE_ROLES).await?;

    if payload.name.is_empty() || payload.name.len() > 100 {
        return Err(AppError::BadRequest("Role name must be 1-100 characters".into()));
    }

    let caller_bits = roles::get_member_permissions(&state.db, guild_id, user_id).await?;
    let caller_perms = Permissions::new(caller_bits);
    let requested_bits = payload.permissions.unwrap_or(0);
    let safe_bits = if caller_perms.has(Permissions::ADMINISTRATOR) {
        requested_bits
    } else {
        requested_bits & caller_bits
    };

    let role_id = pulsar_common::models::snowflake::Snowflake::generate().0;
    let existing = roles::find_by_guild(&state.db, guild_id).await?;
    let position = existing.len() as i32;

    let row = roles::insert(
        &state.db,
        role_id,
        guild_id,
        &payload.name,
        payload.color.unwrap_or(0),
        position,
        safe_bits,
    ).await?;

    info!(role_id = %row.id, guild_id = %guild_id, name = %row.name, "Role created");

    Ok(Json(to_response(row)))
}

pub async fn update_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
    Json(payload): Json<UpdateRole>,
) -> Result<Json<RoleResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;
    let role_id: i64 = role_id.parse().map_err(|_| AppError::BadRequest("Invalid role ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::MANAGE_ROLES).await?;

    let existing_roles = roles::find_by_guild(&state.db, guild_id).await?;
    let current = existing_roles.iter().find(|r| r.id == role_id)
        .ok_or(AppError::NotFound("Role not found".into()))?;

    let name = payload.name.as_deref().unwrap_or(&current.name);
    let color = payload.color.unwrap_or(current.color);

    let caller_bits = roles::get_member_permissions(&state.db, guild_id, user_id).await?;
    let caller_perms = Permissions::new(caller_bits);
    let permissions = if let Some(requested_bits) = payload.permissions {
        if caller_perms.has(Permissions::ADMINISTRATOR) {
            requested_bits
        } else {
            requested_bits & caller_bits
        }
    } else {
        current.permissions
    };

    let row = roles::update(&state.db, role_id, guild_id, name, color, permissions).await?;

    info!(role_id = %role_id, "Role updated");

    Ok(Json(to_response(row)))
}

pub async fn delete_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;
    let role_id: i64 = role_id.parse().map_err(|_| AppError::BadRequest("Invalid role ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::MANAGE_ROLES).await?;

    roles::delete(&state.db, role_id, guild_id).await?;

    info!(role_id = %role_id, "Role deleted");

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn assign_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
    Json(payload): Json<AssignRoleBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;
    let role_id: i64 = role_id.parse().map_err(|_| AppError::BadRequest("Invalid role ID".into()))?;
    let target_id: i64 = payload.user_id.parse().map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::MANAGE_ROLES).await?;

    if !guilds::is_member(&state.db, guild_id, target_id).await? {
        return Err(AppError::NotFound("Target user is not a member of this guild".into()));
    }

    let guild_roles = roles::find_by_guild(&state.db, guild_id).await?;
    let target_role = guild_roles.iter().find(|r| r.id == role_id)
        .ok_or(AppError::NotFound("Role not found in this guild".into()))?;

    let guild = guilds::find_by_id(&state.db, guild_id).await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    if guild.owner_id != user_id {
        let caller_bits = roles::get_member_permissions(&state.db, guild_id, user_id).await?;
        let caller_perms = Permissions::new(caller_bits);
        if !caller_perms.has(Permissions::ADMINISTRATOR) && (target_role.permissions & !caller_bits) != 0 {
            return Err(AppError::Forbidden);
        }
    }

    roles::assign_role(&state.db, guild_id, target_id, role_id).await?;

    info!(role_id = %role_id, target = %target_id, "Role assigned");

    Ok(Json(serde_json::json!({ "assigned": true })))
}

pub async fn unassign_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((guild_id, role_id, target_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let guild_id: i64 = guild_id.parse().map_err(|_| AppError::BadRequest("Invalid guild ID".into()))?;
    let role_id: i64 = role_id.parse().map_err(|_| AppError::BadRequest("Invalid role ID".into()))?;
    let target_id: i64 = target_id.parse().map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    if !guilds::is_member(&state.db, guild_id, user_id).await? {
        return Err(AppError::Forbidden);
    }
    perms::check_permission(&state.db, guild_id, user_id, Permissions::MANAGE_ROLES).await?;

    roles::remove_role(&state.db, guild_id, target_id, role_id).await?;

    info!(role_id = %role_id, target = %target_id, "Role unassigned");

    Ok(Json(serde_json::json!({ "removed": true })))
}