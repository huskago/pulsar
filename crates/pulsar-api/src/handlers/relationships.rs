use axum::{
    extract::{Path, Query, State},
    Json,
};
use pulsar_common::error::AppError;
use pulsar_db::repo::{relationships, users};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::{middleware::auth::AuthUser, state::AppState};
use super::privacy;

#[derive(Debug, Deserialize)]
pub struct CreateRelationship {
    pub user_id: String,
    #[serde(rename = "type")]
    pub kind: String, // "friend" or "block"
}

#[derive(Debug, Deserialize)]
pub struct UpdateRelationship {
    pub action: String, // "accept" or "decline"
}

#[derive(Debug, Deserialize)]
pub struct RelationshipQuery {
    pub kind: Option<String>, // "friend", "pending", "blocked"
}

#[derive(Debug, Serialize)]
pub struct RelationshipResponse {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct MutualFriendsResponse {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

pub async fn create_relationship(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRelationship>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let target_id: i64 = payload.user_id.parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    if user_id == target_id {
        return Err(AppError::BadRequest("Cannot target yourself".into()));
    }

    users::find_by_id(&state.db, target_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    match payload.kind.as_str() {
        "friend" => {
            if relationships::has_pending_request(&state.db, user_id, target_id).await? {
                return Err(AppError::BadRequest("Already friends or request pending".into()));
            }

            if relationships::is_blocked(&state.db, target_id, user_id).await? {
                return Err(AppError::NotFound("User not found".into()));
            }

            if !privacy::can_send_friend_request(&state.db, user_id, target_id).await? {
                return Err(AppError::NotFound("User not found".into()));
            }

            if let Some(existing) = relationships::get_relationship(&state.db, user_id, target_id).await?
                && existing.kind == "pending_incoming" {
                    relationships::accept_friend_request(&state.db, user_id, target_id).await?;
                    info!(user = %user_id, target = %target_id, "Friend request auto-accepted (mutual)");
                    return Ok(Json(serde_json::json!({ "status": "friends" })));
                }

            relationships::send_friend_request(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "Friend request sent");
            Ok(Json(serde_json::json!({ "status": "pending" })))
        }
        "block" => {
            relationships::block_user(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "User blocked");
            Ok(Json(serde_json::json!({ "status": "blocked" })))
        }
        _ => Err(AppError::BadRequest("type must be 'friend' or 'block'".into())),
    }
}

pub async fn update_relationship(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    Json(payload): Json<UpdateRelationship>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let target_id: i64 = target_id.parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    match payload.action.as_str() {
        "accept" => {
            relationships::accept_friend_request(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "Friend request accepted");
            Ok(Json(serde_json::json!({ "status": "friends" })))
        }
        "decline" => {
            relationships::decline_friend_request(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "Friend request declined");
            Ok(Json(serde_json::json!({ "status": "declined" })))
        }
        _ => Err(AppError::BadRequest("action must be 'accept' or 'decline'".into())),
    }
}

pub async fn delete_relationship(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(target_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let target_id: i64 = target_id.parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let rel = relationships::get_relationship(&state.db, user_id, target_id).await?;

    match rel.map(|r| r.kind) {
        Some(kind) if kind == "friend" => {
            relationships::remove_friend(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "Friend removed");
            Ok(Json(serde_json::json!({ "status": "removed" })))
        }
        Some(kind) if kind == "blocked" => {
            relationships::unblock_user(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "User unblocked");
            Ok(Json(serde_json::json!({ "status": "unblocked" })))
        }
        Some(kind) if kind == "pending_outgoing" => {
            relationships::decline_friend_request(&state.db, user_id, target_id).await?;
            info!(user = %user_id, target = %target_id, "Friend request cancelled");
            Ok(Json(serde_json::json!({ "status": "cancelled" })))
        }
        Some(kind) if kind == "pending_incoming" => {
            relationships::decline_friend_request(&state.db, user_id, target_id).await?;
            Ok(Json(serde_json::json!({ "status": "declined" })))
        }
        _ => Err(AppError::NotFound("No relationship found".into())),
    }
}

pub async fn list_relationships(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<RelationshipQuery>,
) -> Result<Json<Vec<RelationshipResponse>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    let kinds: Vec<&str> = match params.kind.as_deref() {
        Some("friend") => vec!["friend"],
        Some("pending") => vec!["pending_incoming", "pending_outgoing"],
        Some("blocked") => vec!["blocked"],
        Some("incoming") => vec!["pending_incoming"],
        Some("outgoing") => vec!["pending_outgoing"],
        _ => vec!["friend"],
    };

    let mut all_rows = Vec::new();
    for kind in kinds {
        let rows = relationships::list_by_kind(&state.db, user_id, kind).await?;
        all_rows.extend(rows);
    }

    let target_ids: Vec<i64> = all_rows.iter().map(|r| r.target_id).collect();
    let user_list = users::find_by_ids(&state.db, &target_ids).await?;
    let user_map: HashMap<i64, _> = user_list.into_iter().map(|u| (u.id, u)).collect();

    let results: Vec<RelationshipResponse> = all_rows.into_iter()
        .filter_map(|row| {
            user_map.get(&row.target_id).map(|u| RelationshipResponse {
                user_id: row.target_id.to_string(),
                username: u.username.clone(),
                avatar_url: u.avatar_url.clone(),
                kind: row.kind,
                created_at: row.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(results))
}

pub async fn get_mutual_friends(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(target_id): Path<String>,
) -> Result<Json<Vec<MutualFriendsResponse>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let target_id: i64 = target_id.parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let mutual_ids = relationships::mutual_friends(&state.db, user_id, target_id).await?;

    let user_list = users::find_by_ids(&state.db, &mutual_ids).await?;
    let user_map: HashMap<i64, _> = user_list.into_iter().map(|u| (u.id, u)).collect();

    let results: Vec<MutualFriendsResponse> = mutual_ids.into_iter()
        .filter_map(|mid| {
            user_map.get(&mid).map(|u| MutualFriendsResponse {
                user_id: mid.to_string(),
                username: u.username.clone(),
                avatar_url: u.avatar_url.clone(),
            })
        })
        .collect();

    Ok(Json(results))
}
