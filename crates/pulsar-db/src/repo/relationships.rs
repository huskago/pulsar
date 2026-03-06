use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct RelationshipRow {
    pub user_id: i64,
    pub target_id: i64,
    pub kind: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn send_friend_request(pool: &PgPool, sender: i64, target: i64) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX: {}", e)))?;

    sqlx::query(
        "INSERT INTO relationships (user_id, target_id, kind)
         VALUES ($1, $2, 'pending_outgoing')
         ON CONFLICT DO NOTHING",
    )
    .bind(sender)
    .bind(target)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert outgoing: {}", e)))?;

    sqlx::query(
        "INSERT INTO relationships (user_id, target_id, kind)
         VALUES ($1, $2, 'pending_incoming')
         ON CONFLICT DO NOTHING",
    )
    .bind(target)
    .bind(sender)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert incoming: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Commit: {}", e)))?;

    Ok(())
}

pub async fn accept_friend_request(
    pool: &PgPool,
    user_id: i64,
    requester_id: i64,
) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX: {}", e)))?;

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM relationships
         WHERE user_id = $1 AND target_id = $2 AND kind = 'pending_incoming'",
    )
    .bind(user_id)
    .bind(requester_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Check pending: {}", e)))?;

    if exists == 0 {
        return Err(AppError::NotFound("No pending friend request".into()));
    }

    sqlx::query(
        "UPDATE relationships SET kind = 'friend'
         WHERE user_id = $1 AND target_id = $2",
    )
    .bind(user_id)
    .bind(requester_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Update 1: {}", e)))?;

    sqlx::query(
        "UPDATE relationships SET kind = 'friend'
         WHERE user_id = $1 AND target_id = $2",
    )
    .bind(requester_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Update 2: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Commit: {}", e)))?;

    Ok(())
}

pub async fn decline_friend_request(
    pool: &PgPool,
    user_id: i64,
    other_id: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM relationships
         WHERE (user_id = $1 AND target_id = $2)
            OR (user_id = $2 AND target_id = $1)",
    )
    .bind(user_id)
    .bind(other_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Decline: {}", e)))?;

    Ok(())
}

pub async fn remove_friend(pool: &PgPool, user_id: i64, friend_id: i64) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM relationships
         WHERE ((user_id = $1 AND target_id = $2) OR (user_id = $2 AND target_id = $1))
           AND kind = 'friend'",
    )
    .bind(user_id)
    .bind(friend_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Remove friend: {}", e)))?;

    Ok(())
}

pub async fn block_user(pool: &PgPool, user_id: i64, target_id: i64) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX: {}", e)))?;

    sqlx::query(
        "DELETE FROM relationships
         WHERE (user_id = $1 AND target_id = $2)
            OR (user_id = $2 AND target_id = $1)",
    )
    .bind(user_id)
    .bind(target_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Clear: {}", e)))?;

    sqlx::query(
        "INSERT INTO relationships (user_id, target_id, kind)
         VALUES ($1, $2, 'blocked')",
    )
    .bind(user_id)
    .bind(target_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Block: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Commit: {}", e)))?;

    Ok(())
}

pub async fn unblock_user(pool: &PgPool, user_id: i64, target_id: i64) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM relationships
         WHERE user_id = $1 AND target_id = $2 AND kind = 'blocked'",
    )
    .bind(user_id)
    .bind(target_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Unblock: {}", e)))?;

    Ok(())
}

pub async fn is_friend(pool: &PgPool, user_a: i64, user_b: i64) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM relationships
         WHERE user_id = $1 AND target_id = $2 AND kind = 'friend'",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(count > 0)
}

pub async fn is_blocked(pool: &PgPool, blocker: i64, target: i64) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM relationships
         WHERE user_id = $1 AND target_id = $2 AND kind = 'blocked'",
    )
    .bind(blocker)
    .bind(target)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(count > 0)
}

pub async fn has_mutual_friends(pool: &PgPool, user_a: i64, user_b: i64) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM relationships r1
         INNER JOIN relationships r2 ON r1.target_id = r2.user_id
         WHERE r1.user_id = $1 AND r1.kind = 'friend'
           AND r2.target_id = $2 AND r2.kind = 'friend'",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(count > 0)
}

pub async fn share_guild(pool: &PgPool, user_a: i64, user_b: i64) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM guild_members gm1
         INNER JOIN guild_members gm2 ON gm1.guild_id = gm2.guild_id
         WHERE gm1.user_id = $1 AND gm2.user_id = $2",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(count > 0)
}

pub async fn mutual_friends(pool: &PgPool, user_a: i64, user_b: i64) -> Result<Vec<i64>, AppError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT r1.target_id FROM relationships r1
         INNER JOIN relationships r2 ON r1.target_id = r2.target_id
         WHERE r1.user_id = $1 AND r1.kind = 'friend'
           AND r2.user_id = $2 AND r2.kind = 'friend'",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn list_by_kind(
    pool: &PgPool,
    user_id: i64,
    kind: &str,
) -> Result<Vec<RelationshipRow>, AppError> {
    sqlx::query_as::<_, RelationshipRow>(
        "SELECT * FROM relationships WHERE user_id = $1 AND kind = $2 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .bind(kind)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn get_relationship(
    pool: &PgPool,
    user_id: i64,
    target_id: i64,
) -> Result<Option<RelationshipRow>, AppError> {
    sqlx::query_as::<_, RelationshipRow>(
        "SELECT * FROM relationships WHERE user_id = $1 AND target_id = $2",
    )
    .bind(user_id)
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn has_pending_request(
    pool: &PgPool,
    sender: i64,
    target: i64,
) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM relationships
         WHERE user_id = $1 AND target_id = $2
           AND kind IN ('pending_outgoing', 'friend')",
    )
    .bind(sender)
    .bind(target)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(count > 0)
}
