use chrono::{DateTime, Utc};
use pulsar_common::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: i64,
    pub device_name: String,
    #[serde(skip)]
    pub token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    #[serde(skip)]
    pub current_jti: Option<String>,
}

pub async fn insert(
    pool: &PgPool,
    user_id: i64,
    device_name: &str,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO sessions (user_id, device_name, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(user_id)
    .bind(device_name)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions insert: {}", e)))?;
    Ok(row.0)
}

pub async fn find_by_token_hash(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<Option<SessionRow>, AppError> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT * FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions find: {}", e)))
}

pub async fn delete(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions delete: {}", e)))?;
    Ok(())
}

pub async fn delete_all_for_user(pool: &PgPool, user_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions delete_all: {}", e)))?;
    Ok(())
}

pub async fn list_for_user(pool: &PgPool, user_id: i64) -> Result<Vec<SessionRow>, AppError> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT * FROM sessions WHERE user_id = $1 AND expires_at > NOW() ORDER BY last_used DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions list: {}", e)))
}

pub async fn delete_owned(pool: &PgPool, session_id: Uuid, user_id: i64) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions delete_owned: {}", e)))?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_owned_returning_jti(
    pool: &PgPool,
    session_id: Uuid,
    user_id: i64,
) -> Result<(bool, Option<String>), AppError> {
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "DELETE FROM sessions WHERE id = $1 AND user_id = $2 RETURNING current_jti",
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions delete_owned_jti: {}", e)))?;
    match row {
        None => Ok((false, None)),
        Some((jti,)) => Ok((true, jti)),
    }
}

pub async fn update_jti(pool: &PgPool, session_id: Uuid, jti: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET current_jti = $1 WHERE id = $2")
        .bind(jti)
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions update_jti: {}", e)))?;
    Ok(())
}

pub async fn list_jtis_for_user(pool: &PgPool, user_id: i64) -> Result<Vec<String>, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT current_jti FROM sessions WHERE user_id = $1 AND current_jti IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions list_jtis: {}", e)))?;
    Ok(rows.into_iter().map(|(jti,)| jti).collect())
}

pub async fn touch(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET last_used = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions touch: {}", e)))?;
    Ok(())
}

pub async fn update_token_hash(
    pool: &PgPool,
    session_id: Uuid,
    new_hash: &[u8],
) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET token_hash = $1, last_used = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("sessions update_token_hash: {}", e)))?;
    Ok(())
}
