use pulsar_common::error::AppError;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildRow {
    pub id: i64,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildMemberRow {
    pub guild_id: i64,
    pub user_id: i64,
    pub nickname: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

pub async fn insert(
    pool: &PgPool,
    id: i64,
    name: &str,
    owner_id: i64,
) -> Result<GuildRow, AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX begin: {}", e)))?;

    let guild = sqlx::query_as::<_, GuildRow>(
        "INSERT INTO guilds (id, name, owner_id)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(owner_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert guild: {}", e)))?;

    sqlx::query("INSERT INTO guild_members (guild_id, user_id) VALUES ($1, $2)")
        .bind(id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert member: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TX commit: {}", e)))?;

    Ok(guild)
}

pub async fn find_by_user(pool: &PgPool, user_id: i64) -> Result<Vec<GuildRow>, AppError> {
    sqlx::query_as::<_, GuildRow>(
        "SELECT g.* FROM guilds g
         INNER JOIN guild_members gm ON g.id = gm.guild_id
         WHERE gm.user_id = $1
         ORDER BY g.created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<GuildRow>, AppError> {
    sqlx::query_as::<_, GuildRow>("SELECT * FROM guilds WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn is_member(pool: &PgPool, guild_id: i64, user_id: i64) -> Result<bool, AppError> {
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id = $1 AND user_id = $2)",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(row)
}
