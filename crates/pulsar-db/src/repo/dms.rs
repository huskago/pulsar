use sqlx::PgPool;
use pulsar_common::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannelRow {
    pub channel_id: i64,
    pub user_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmConversation {
    pub channel_id: i64,
    pub other_user_id: i64,
    pub other_username: String,
    pub other_avatar_url: Option<String>,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn find_between(
    pool: &PgPool,
    user_a: i64,
    user_b: i64,
) -> Result<Option<i64>, AppError> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT d1.channel_id FROM dm_channels d1
         INNER JOIN dm_channels d2 ON d1.channel_id = d2.channel_id
         WHERE d1.user_id = $1 AND d2.user_id = $2
         LIMIT 1"
    )
        .bind(user_a)
        .bind(user_b)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(row)
}

pub async fn create(
    pool: &PgPool,
    channel_id: i64,
    user_a: i64,
    user_b: i64,
) -> Result<i64, AppError> {
    sqlx::query(
        "INSERT INTO channels (id, guild_id, name, kind, position)
         VALUES ($1, NULL, 'dm', 'dm', 0)"
    )
        .bind(channel_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create DM channel: {}", e)))?;

    sqlx::query(
        "INSERT INTO dm_channels (channel_id, user_id) VALUES ($1, $2), ($1, $3)"
    )
        .bind(channel_id)
        .bind(user_a)
        .bind(user_b)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Add DM participants: {}", e)))?;

    Ok(channel_id)
}

pub async fn list_conversations(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<DmConversation>, AppError> {
    sqlx::query_as::<_, DmConversation>(
        "SELECT
            d1.channel_id,
            d2.user_id AS other_user_id,
            u.username AS other_username,
            u.avatar_url AS other_avatar_url,
            (SELECT MAX(created_at) FROM messages WHERE channel_id = d1.channel_id) AS last_message_at
         FROM dm_channels d1
         INNER JOIN dm_channels d2 ON d1.channel_id = d2.channel_id AND d2.user_id != d1.user_id
         INNER JOIN users u ON u.id = d2.user_id
         WHERE d1.user_id = $1
         ORDER BY last_message_at DESC NULLS LAST"
    )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn is_participant(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<bool, AppError> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dm_channels WHERE channel_id = $1 AND user_id = $2"
    )
        .bind(channel_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(row > 0)
}

pub async fn get_participants(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Vec<i64>, AppError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM dm_channels WHERE channel_id = $1"
    )
        .bind(channel_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}