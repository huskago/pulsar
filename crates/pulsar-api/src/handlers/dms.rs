use axum::{
    extract::State,
    Json,
};
use pulsar_common::error::AppError;
use pulsar_db::repo::{dms, users};
use serde::{Deserialize, Serialize};
use tracing::info;
use super::privacy as privacy_check;

use crate::{middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Deserialize)]
pub struct OpenDm {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct DmResponse {
    pub channel_id: String,
    pub other_user: DmUserInfo,
}

#[derive(Debug, Serialize)]
pub struct DmConversationResponse {
    pub channel_id: String,
    pub other_user: DmUserInfo,
    pub last_message_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DmUserInfo {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupDm {
    pub user_ids: Vec<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupDmResponse {
    pub channel_id: String,
    pub name: Option<String>,
    pub participants: Vec<DmUserInfo>,
}

pub async fn open_dm(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<OpenDm>,
) -> Result<Json<DmResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let target_id: i64 = payload.user_id.parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    if user_id == target_id {
        return Err(AppError::BadRequest("Cannot DM yourself".into()));
    }

    if !privacy_check::can_dm(&state.db, user_id, target_id).await? {
        return Err(AppError::NotFound("User not found".into()));
    }

    let target = users::find_by_id(&state.db, target_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let channel_id = match dms::find_between(&state.db, user_id, target_id).await? {
        Some(id) => id,
        None => {
            let new_id = pulsar_common::models::snowflake::Snowflake::generate().0;
            dms::create(&state.db, new_id, user_id, target_id).await?;
            info!(channel_id = %new_id, user_a = %user_id, user_b = %target_id, "DM created");
            new_id
        }
    };

    Ok(Json(DmResponse {
        channel_id: channel_id.to_string(),
        other_user: DmUserInfo {
            id: target.id.to_string(),
            username: target.username,
            avatar_url: target.avatar_url,
        },
    }))
}

pub async fn list_dms(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DmConversationResponse>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    let convos = dms::list_conversations(&state.db, user_id).await?;

    let response: Vec<DmConversationResponse> = convos
        .into_iter()
        .map(|c| DmConversationResponse {
            channel_id: c.channel_id.to_string(),
            other_user: DmUserInfo {
                id: c.other_user_id.to_string(),
                username: c.other_username,
                avatar_url: c.other_avatar_url,
            },
            last_message_at: c.last_message_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

pub async fn create_group_dm(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateGroupDm>,
) -> Result<Json<GroupDmResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    if payload.user_ids.is_empty() || payload.user_ids.len() > 9 {
        return Err(AppError::BadRequest("Group DM needs 1-9 other participants".into()));
    }

    let mut participant_ids: Vec<i64> = vec![user_id];
    let mut participant_infos: Vec<DmUserInfo> = Vec::new();

    for uid_str in &payload.user_ids {
        let uid: i64 = uid_str.parse()
            .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

        if uid == user_id { continue; }
        if participant_ids.contains(&uid) { continue; }

        let target = users::find_by_id(&state.db, uid)
            .await?
            .ok_or(AppError::NotFound(format!("User {} not found", uid)))?;

        if !privacy_check::can_dm(&state.db, user_id, uid).await? {
            return Err(AppError::NotFound(format!("User {} not found", uid)));
        }

        participant_ids.push(uid);
        participant_infos.push(DmUserInfo {
            id: target.id.to_string(),
            username: target.username,
            avatar_url: target.avatar_url,
        });
    }

    if participant_ids.len() < 2 {
        return Err(AppError::BadRequest("Need at least 1 other participant".into()));
    }

    let channel_id = pulsar_common::models::snowflake::Snowflake::generate().0;

    sqlx::query(
        "INSERT INTO channels (id, guild_id, name, kind, position)
         VALUES ($1, NULL, $2, 'dm', 0)"
    )
        .bind(channel_id)
        .bind(payload.name.as_deref().unwrap_or("Group DM"))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create group channel: {}", e)))?;

    for &pid in &participant_ids {
        sqlx::query(
            "INSERT INTO dm_channels (channel_id, user_id, is_group)
             VALUES ($1, $2, TRUE)"
        )
            .bind(channel_id)
            .bind(pid)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Add participant: {}", e)))?;
    }

    sqlx::query(
        "INSERT INTO group_dm_info (channel_id, name, owner_id)
         VALUES ($1, $2, $3)"
    )
        .bind(channel_id)
        .bind(payload.name.as_deref())
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create group info: {}", e)))?;

    info!(channel_id = %channel_id, participants = %participant_ids.len(), "Group DM created");

    Ok(Json(GroupDmResponse {
        channel_id: channel_id.to_string(),
        name: payload.name,
        participants: participant_infos,
    }))
}