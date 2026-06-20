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

pub async fn insert(
    pool: &PgPool,
    id: i64,
    message_id: i64,
    filename: &str,
    content_type: &str,
    size: i64,
    storage_key: &str,
    url: &str,
) -> Result<AttachmentRow, AppError> {
    sqlx::query_as::<_, AttachmentRow>(
        "INSERT INTO attachments (id, message_id, filename, content_type, size, storage_key, url)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
    )
    .bind(id)
    .bind(message_id)
    .bind(filename)
    .bind(content_type)
    .bind(size)
    .bind(storage_key)
    .bind(url)
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
