use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSettingsRow {
    pub dm_privacy: String,
    pub friend_request_privacy: String,
}

pub async fn insert(
    pool: &PgPool,
    id: i64,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<UserRow, AppError> {
    sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (id, username, email, password_hash)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
    )
    .bind(id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            AppError::BadRequest("Username or email already taken".into())
        }
        _ => AppError::Internal(anyhow::anyhow!("DB error: {}", e)),
    })
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, AppError> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<UserRow>, AppError> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn update_status(pool: &PgPool, id: i64, status: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE users SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(())
}

pub async fn get_settings(pool: &PgPool, user_id: i64) -> Result<UserSettingsRow, AppError> {
    sqlx::query_as::<_, UserSettingsRow>(
        "SELECT dm_privacy, friend_request_privacy FROM users WHERE id = $1"
    )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn update_settings(
    pool: &PgPool,
    user_id: i64,
    dm_privacy: &str,
    friend_request_privacy: &str,
) -> Result<UserSettingsRow, AppError> {
    sqlx::query_as::<_, UserSettingsRow>(
        "UPDATE users SET dm_privacy = $2, friend_request_privacy = $3
         WHERE id = $1
         RETURNING dm_privacy, friend_request_privacy"
    )
        .bind(user_id)
        .bind(dm_privacy)
        .bind(friend_request_privacy)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Update settings: {}", e)))
}
