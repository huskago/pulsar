use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AttachmentRow {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub storage_key: String,
    pub url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NewAttachment<'a> {
    pub id: i64,
    pub message_id: i64,
    pub filename: &'a str,
    pub content_type: &'a str,
    pub size: i64,
    pub storage_key: &'a str,
    pub url: &'a str,
}

pub async fn insert(pool: &PgPool, att: NewAttachment<'_>) -> Result<AttachmentRow, AppError> {
    sqlx::query_as::<_, AttachmentRow>(
        "INSERT INTO attachments (id, message_id, filename, content_type, size, storage_key, url)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
    )
    .bind(att.id)
    .bind(att.message_id)
    .bind(att.filename)
    .bind(att.content_type)
    .bind(att.size)
    .bind(att.storage_key)
    .bind(att.url)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert attachment: {}", e)))
}

pub async fn find_by_message(
    pool: &PgPool,
    message_id: i64,
) -> Result<Vec<AttachmentRow>, AppError> {
    sqlx::query_as::<_, AttachmentRow>(
        "SELECT * FROM attachments WHERE message_id = $1 ORDER BY created_at",
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn delete_by_message(pool: &PgPool, message_id: i64) -> Result<Vec<AttachmentRow>, AppError> {
    sqlx::query_as::<_, AttachmentRow>(
        "DELETE FROM attachments WHERE message_id = $1 RETURNING *",
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Delete attachments: {}", e)))
}

pub async fn find_by_messages(
    pool: &PgPool,
    message_ids: &[i64],
) -> Result<Vec<AttachmentRow>, AppError> {
    sqlx::query_as::<_, AttachmentRow>(
        "SELECT * FROM attachments WHERE message_id = ANY($1) ORDER BY created_at",
    )
    .bind(message_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn delete_by_message_ids(pool: &PgPool, message_ids: &[i64]) -> Result<(), AppError> {
    if message_ids.is_empty() {
        return Ok(());
    }
    sqlx::query("DELETE FROM attachments WHERE message_id = ANY($1)")
        .bind(message_ids)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("delete_by_message_ids: {}", e)))?;
    Ok(())
}
