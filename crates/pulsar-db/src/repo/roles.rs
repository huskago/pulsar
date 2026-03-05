use sqlx::PgPool;
use pulsar_common::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct RoleRow {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: i64,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create the @everyone role for a new guild
pub async fn create_default_role(
    pool: &PgPool,
    role_id: i64,
    guild_id: i64,
    default_perms: i64,
) -> Result<RoleRow, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "INSERT INTO roles (id, guild_id, name, color, position, permissions, is_default)
             VALUES ($1, $2, 'everyone', 0, 0, $3, TRUE)
             RETURNING *"
    )
    .bind(role_id)
    .bind(guild_id)
    .bind(default_perms)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Create default role: {}", e)))
}

pub async fn insert(
    pool: &PgPool,
    id: i64,
    guild_id: i64,
    name: &str,
    color: i32,
    position: i32,
    permissions: i64,
) -> Result<RoleRow, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "INSERT INTO roles (id, guild_id, name, color, position, permissions)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING *"
    )
    .bind(id)
    .bind(guild_id)
    .bind(name)
    .bind(color)
    .bind(position)
    .bind(permissions)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Insert role: {}", e)))
}

pub async fn update(
    pool: &PgPool,
    role_id: i64,
    name: &str,
    color: i32,
    permissions: i64,
) -> Result<RoleRow, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "UPDATE roles SET name = $2, color = $3, permissions = $4
         WHERE id = $1
         RETURNING *"
    )
    .bind(role_id)
    .bind(name)
    .bind(color)
    .bind(permissions)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Update role: {}", e)))
}

pub async fn delete(pool: &PgPool, role_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM roles WHERE id = $1 AND is_default = FALSE")
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Delete role: {}", e)))?;
    Ok(())
}

pub async fn find_by_guild(pool: &PgPool, guild_id: i64) -> Result<Vec<RoleRow>, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "SELECT * FROM roles WHERE guild_id = $1 ORDER BY position ASC"
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

pub async fn find_default(pool: &PgPool, guild_id: i64) -> Result<Option<RoleRow>, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "SELECT * FROM roles WHERE guild_id = $1 AND is_default = TRUE"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}

/// Retrieve the combined permissions of a member (all their roles + @everyone)
pub async fn get_member_permissions(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<i64, AppError> {
    // Combine: @everyone permissions OR all assigned role permissions
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
            (SELECT bit_or(r.permissions) FROM roles r
             WHERE r.guild_id = $1 AND (
                 r.is_default = TRUE
                 OR r.id IN (SELECT role_id FROM member_roles WHERE guild_id = $1 AND user_id = $2)
             )),
            0
         )"
    )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(row)
}

/// Assign a role to a member
pub async fn assign_role(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
    role_id: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO member_roles (guild_id, user_id, role_id)
         VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING"
    )
        .bind(guild_id)
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Assign role: {}", e)))?;
    Ok(())
}

/// Remove a role from a member
pub async fn remove_role(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
    role_id: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM member_roles WHERE guild_id = $1 AND user_id = $2 AND role_id = $3"
    )
        .bind(guild_id)
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Remove role: {}", e)))?;
    Ok(())
}

/// Retrieve the roles of a member
pub async fn get_member_roles(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<Vec<RoleRow>, AppError> {
    sqlx::query_as::<_, RoleRow>(
        "SELECT r.* FROM roles r
         WHERE r.guild_id = $1 AND (
             r.is_default = TRUE
             OR r.id IN (SELECT role_id FROM member_roles WHERE guild_id = $1 AND user_id = $2)
         )
         ORDER BY r.position ASC"
    )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))
}