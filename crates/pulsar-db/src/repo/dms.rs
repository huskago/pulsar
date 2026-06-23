use sqlx::PgPool;
use pulsar_common::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannelRow {
    pub channel_id: i64,
    pub user_id: i64,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
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
           AND d1.is_group = FALSE AND d2.is_group = FALSE
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
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX: {}", e)))?;

    sqlx::query(
        "INSERT INTO channels (id, guild_id, name, kind, position)
         VALUES ($1, NULL, 'dm', 'dm', 0)"
    )
        .bind(channel_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create DM channel: {}", e)))?;

    sqlx::query(
        "INSERT INTO dm_channels (channel_id, user_id) VALUES ($1, $2), ($1, $3)"
    )
        .bind(channel_id)
        .bind(user_a)
        .bind(user_b)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Add DM participants: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Commit: {}", e)))?;

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
            d1.last_message_at
         FROM dm_channels d1
         INNER JOIN dm_channels d2 ON d1.channel_id = d2.channel_id AND d2.user_id != d1.user_id
         INNER JOIN users u ON u.id = d2.user_id
         WHERE d1.user_id = $1 AND d1.is_group = FALSE
         ORDER BY d1.last_message_at DESC NULLS LAST"
    )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn update_last_message_at(pool: &PgPool, channel_id: i64) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE dm_channels SET last_message_at = NOW() WHERE channel_id = $1"
    )
        .bind(channel_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    Ok(())
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

pub async fn find_group_between(
    pool: &PgPool,
    participant_ids: &[i64],
) -> Result<Option<i64>, AppError> {
    let n = participant_ids.len() as i64;
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT dc.channel_id
         FROM dm_channels dc
         JOIN channels c ON c.id = dc.channel_id
         WHERE dc.is_group = TRUE
         GROUP BY dc.channel_id
         HAVING
             COUNT(*) = $1
             AND COUNT(*) FILTER (WHERE dc.user_id = ANY($2::bigint[])) = $1
         LIMIT 1",
    )
    .bind(n)
    .bind(participant_ids)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    Ok(row)
}

pub async fn create_group(
    pool: &PgPool,
    channel_id: i64,
    name: Option<&str>,
    owner_id: i64,
    participant_ids: &[i64],
    sealed_dek: &[u8],
) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX begin: {}", e)))?;

    sqlx::query(
        "INSERT INTO channels (id, guild_id, name, kind, position)
         VALUES ($1, NULL, $2, 'dm', 0)",
    )
    .bind(channel_id)
    .bind(name.unwrap_or("Group DM"))
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Create group channel: {}", e)))?;

    for &pid in participant_ids {
        sqlx::query(
            "INSERT INTO dm_channels (channel_id, user_id, is_group) VALUES ($1, $2, TRUE)",
        )
        .bind(channel_id)
        .bind(pid)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Add participant: {}", e)))?;
    }

    sqlx::query(
        "INSERT INTO group_dm_info (channel_id, name, owner_id) VALUES ($1, $2, $3)",
    )
    .bind(channel_id)
    .bind(name)
    .bind(owner_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Create group info: {}", e)))?;

    sqlx::query(
        "INSERT INTO channel_keys (channel_id, sealed_dek)
         VALUES ($1, $2)
         ON CONFLICT (channel_id) DO UPDATE SET sealed_dek = EXCLUDED.sealed_dek",
    )
    .bind(channel_id)
    .bind(sealed_dek)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("channel_keys upsert: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX commit: {}", e)))?;

    Ok(())
}