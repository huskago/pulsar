use pulsar_common::{error::AppError, permissions::Permissions};
use pulsar_db::repo::{guilds, roles};
use sqlx::PgPool;

/// Checks whether a user has permission in a guild.
/// The owner bypasses all permissions.
pub async fn check_permission(
    db: &PgPool,
    guild_id: i64,
    user_id: i64,
    required: i64,
) -> Result<(), AppError> {
    let guild = guilds::find_by_id(db, guild_id)
        .await?
        .ok_or(AppError::NotFound("Guild not found".into()))?;

    if guild.owner_id == user_id {
        return Ok(());
    }

    // Check the combined permissions of roles
    let perms_bits = roles::get_member_permissions(db, guild_id, user_id).await?;
    let perms = Permissions::new(perms_bits);

    if perms.has(required) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}