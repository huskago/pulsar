use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InviteRow {
    pub code: String,
    pub guild_id: i64,
    pub creator_id: i64,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn insert(
    pool: &PgPool,
    code: &str,
    guild_id: i64,
    creator_id: i64,
    max_uses: Option<i32>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<InviteRow, AppError> {
    sqlx::query_as::<_, InviteRow>(
        "INSERT INTO invites (code, guild_id, creator_id, max_uses, expires_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
    )
    .bind(code)
    .bind(guild_id)
    .bind(creator_id)
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert invite: {}", e)))
}

pub async fn find_by_code(pool: &PgPool, code: &str) -> Result<Option<InviteRow>, AppError> {
    sqlx::query_as::<_, InviteRow>("SELECT * FROM invites WHERE code = $1")
        .bind(code)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn find_by_guild(pool: &PgPool, guild_id: i64) -> Result<Vec<InviteRow>, AppError> {
    sqlx::query_as::<_, InviteRow>(
        "SELECT * FROM invites WHERE guild_id = $1 ORDER BY expires_at DESC",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn use_invite(pool: &PgPool, code: &str) -> Result<bool, AppError> {
    let result = sqlx::query_scalar::<_, i64>(
        "UPDATE invites SET uses = uses + 1
         WHERE code = $1
           AND (max_uses IS NULL OR uses < max_uses)
           AND (expires_at IS NULL OR expires_at > NOW())
         RETURNING guild_id",
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(result.is_some())
}

pub async fn join_atomically(pool: &PgPool, code: &str, user_id: i64) -> Result<i64, AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX begin: {}", e)))?;

    let guild_id = sqlx::query_scalar::<_, i64>(
        "UPDATE invites SET uses = uses + 1
         WHERE code = $1
           AND (max_uses IS NULL OR uses < max_uses)
           AND (expires_at IS NULL OR expires_at > NOW())
         RETURNING guild_id",
    )
    .bind(code)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("use_invite: {}", e)))?
    .ok_or_else(|| AppError::BadRequest("Invite is no longer valid".into()))?;

    let already_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM guild_members WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("is_member: {}", e)))?;

    if already_member > 0 {
        tx.rollback()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("rollback: {}", e)))?;
        return Err(AppError::BadRequest("Already a member of this guild".into()));
    }

    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("add_member: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("commit: {}", e)))?;

    Ok(guild_id)
}

pub async fn delete(pool: &PgPool, code: &str, guild_id: i64) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM invites WHERE code = $1 AND guild_id = $2")
        .bind(code)
        .bind(guild_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(result.rows_affected() > 0)
}
