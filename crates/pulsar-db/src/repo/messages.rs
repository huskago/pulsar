use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageRow {
    pub id: i64,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub edited_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn insert(
    pool: &PgPool,
    id: i64,
    channel_id: i64,
    author_id: i64,
    content: &str,
) -> Result<MessageRow, AppError> {
    sqlx::query_as::<_, MessageRow>(
        "INSERT INTO messages (id, channel_id, author_id, content)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(id)
    .bind(channel_id)
    .bind(author_id)
    .bind(content)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert message: {}", e)))
}

pub async fn find_by_channel(
    pool: &PgPool,
    channel_id: i64,
    limit: i64,
    before: Option<i64>,
) -> Result<Vec<MessageRow>, AppError> {
    match before {
        Some(before_id) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages
                 WHERE channel_id = $1 AND id < $2
                 ORDER BY id ASC
                 LIMIT $3",
            )
            .bind(channel_id)
            .bind(before_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages
                 WHERE channel_id = $1
                 ORDER BY id ASC
                 LIMIT $2",
            )
            .bind(channel_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}
