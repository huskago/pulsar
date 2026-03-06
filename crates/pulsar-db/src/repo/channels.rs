use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelRow {
    pub id: i64,
    pub guild_id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub position: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn insert(
    pool: &PgPool,
    id: i64,
    guild_id: i64,
    name: &str,
    kind: &str,
    position: i32,
) -> Result<ChannelRow, AppError> {
    sqlx::query_as::<_, ChannelRow>(
        "INSERT INTO channels (id, guild_id, name, kind, position)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(id)
    .bind(guild_id)
    .bind(name)
    .bind(kind)
    .bind(position)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert channel: {}", e)))
}

pub async fn find_by_guild(pool: &PgPool, guild_id: i64) -> Result<Vec<ChannelRow>, AppError> {
    sqlx::query_as::<_, ChannelRow>(
        "SELECT * FROM channels WHERE guild_id = $1 ORDER BY position, created_at",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<ChannelRow>, AppError> {
    sqlx::query_as::<_, ChannelRow>("SELECT * FROM channels WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}
