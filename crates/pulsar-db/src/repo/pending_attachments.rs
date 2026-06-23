use pulsar_common::error::AppError;
use sqlx::PgPool;

pub async fn insert(
    pool: &PgPool,
    user_id: i64,
    channel_id: i64,
    storage_key: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO pending_attachments (user_id, channel_id, storage_key)
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id, storage_key) DO NOTHING",
    )
    .bind(user_id)
    .bind(channel_id)
    .bind(storage_key)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("pending_attachments insert: {}", e)))?;
    Ok(())
}

pub async fn take(pool: &PgPool, user_id: i64, storage_key: &str) -> Result<bool, AppError> {
    let result = sqlx::query(
        "DELETE FROM pending_attachments WHERE user_id = $1 AND storage_key = $2",
    )
    .bind(user_id)
    .bind(storage_key)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("pending_attachments take: {}", e)))?;
    Ok(result.rows_affected() > 0)
}
