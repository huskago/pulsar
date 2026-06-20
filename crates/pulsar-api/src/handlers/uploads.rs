use axum::extract::{Path, State};
use axum_extra::extract::Multipart;
use pulsar_common::error::AppError;
use pulsar_db::repo::{channel_keys, channels, dms, guilds};
use serde::Serialize;
use tracing::info;

use crate::{middleware::auth::AuthUser, state::AppState};

const MAX_FILE_SIZE: usize = 25 * 1024 * 1024;
const PRESIGNED_TTL_SECS: u64 = 900; // 15 minutes

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub key: String,
    pub url: String,
}

pub async fn upload_file(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<axum::Json<UploadResponse>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    let mut channel_id: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "channel_id" => {
                channel_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("Bad field: {}", e)))?,
                );
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?;
                if data.len() > MAX_FILE_SIZE {
                    return Err(AppError::BadRequest(format!(
                        "File too large. Max {} MB",
                        MAX_FILE_SIZE / 1024 / 1024
                    )));
                }
                file_data = Some(data.to_vec());
            }
            _ => {}
        }
    }

    let channel_id_str = channel_id.ok_or(AppError::BadRequest("Missing channel_id".into()))?;
    let file_data = file_data.ok_or(AppError::BadRequest("Missing file".into()))?;
    let file_name = file_name.unwrap_or_else(|| "unnamed".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let channel_id_num: i64 = channel_id_str
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel_id".into()))?;

    let channel = channels::find_by_id(&state.db, channel_id_num)
        .await?
        .ok_or(AppError::NotFound("Channel not found".into()))?;

    if channel.kind == "dm" {
        if !dms::is_participant(&state.db, channel_id_num, user_id).await? {
            return Err(AppError::Forbidden);
        }
    } else if let Some(gid) = channel.guild_id {
        if !guilds::is_member(&state.db, gid, user_id).await? {
            return Err(AppError::Forbidden);
        }
    } else {
        return Err(AppError::Forbidden);
    }

    // Fetch the channel DEK (Redis cache -> PostgreSQL)
    let dek = get_channel_dek(&state, channel_id_num).await?;

    // Encrypt the file with the channel DEK
    let encrypted = state
        .crypto
        .encrypt_file(&dek, &file_data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("File encryption failed: {}", e)))?;

    // Upload the encrypted bytes
    let result = state
        .storage
        .upload(&channel_id_str, &file_name, &content_type, encrypted)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Upload failed: {}", e)))?;

    // Generate a presigned URL (15 min)
    let presigned_url = state
        .storage
        .generate_presigned_url(&result.key, PRESIGNED_TTL_SECS)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Presign failed: {}", e)))?;

    let attachment_id = pulsar_common::models::snowflake::Snowflake::generate().0;

    info!(
        filename = %file_name,
        size = %result.size,
        key = %result.key,
        "File encrypted and uploaded"
    );

    Ok(axum::Json(UploadResponse {
        id: attachment_id.to_string(),
        filename: file_name,
        content_type,
        size: result.size as i64,
        key: result.key,
        url: presigned_url,
    }))
}

/// Fetch the DEK for a channel: Redis cache -> PostgreSQL -> error if absent.
pub async fn get_channel_dek(
    state: &AppState,
    channel_id: i64,
) -> Result<pulsar_crypto::Dek, AppError> {
    // 1. Try Redis cache
    if let Ok(Some(sealed)) = crate::redis_client::get_sealed_dek(&state.redis, channel_id).await {
        return state
            .crypto
            .open_dek(&sealed)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("DEK open failed: {}", e)));
    }

    // 2. Read from PostgreSQL
    let sealed = channel_keys::find(&state.db, channel_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No DEK for channel {}", channel_id)))?;

    // 3. Cache in Redis
    let _ = crate::redis_client::set_sealed_dek(&state.redis, channel_id, &sealed).await;

    state
        .crypto
        .open_dek(&sealed)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DEK open failed: {}", e)))
}

pub async fn get_attachment_url(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let _ = auth;
    let url = state
        .storage
        .generate_presigned_url(&key, PRESIGNED_TTL_SECS)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Presign failed: {}", e)))?;
    Ok(axum::Json(serde_json::json!({ "url": url })))
}
