use axum::extract::State;
use axum_extra::extract::Multipart;
use pulsar_common::error::AppError;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::{middleware::auth::AuthUser, state::AppState};

const MAX_FILE_SIZE: usize = 25 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub url: String,
}

// POST /channels/:channel_id/upload
pub async fn upload_file(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<axum::Json<UploadResponse>, AppError> {
    let _user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

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

    let channel_id = channel_id.ok_or(AppError::BadRequest("Missing channel_id".into()))?;
    let file_data = file_data.ok_or(AppError::BadRequest("Missing file".into()))?;
    let file_name = file_name.unwrap_or_else(|| "unnamed".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let result = state
        .storage
        .upload(&channel_id, &file_name, &content_type, file_data)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Upload failed: {}", e)))?;

    let attachment_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    info!(
        filename = %file_name,
        size = %result.size,
        content_type = %content_type,
        "File uploaded to storage"
    );

    Ok(axum::Json(UploadResponse {
        id: attachment_id.to_string(),
        filename: file_name,
        content_type,
        size: result.size as i64,
        url: result.url,
    }))
}
