use pulsar_common::error::AppError;
use sqlx::PgPool;

pub async fn upsert(pool: &PgPool, channel_id: i64, sealed_dek: &[u8]) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO channel_keys (channel_id, sealed_dek)
         VALUES ($1, $2)
         ON CONFLICT (channel_id) DO UPDATE SET sealed_dek = EXCLUDED.sealed_dek",
    )
    .bind(channel_id)
    .bind(sealed_dek)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("channel_keys upsert: {}", e)))?;
    Ok(())
}

pub async fn find(pool: &PgPool, channel_id: i64) -> Result<Option<Vec<u8>>, AppError> {
    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT sealed_dek FROM channel_keys WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("channel_keys find: {}", e)))?;
    Ok(row.map(|(dek,)| dek))
}
