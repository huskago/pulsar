use axum::{extract::State, Json};
use pulsar_common::{
    error::AppError,
    models::{snowflake::Snowflake, user::{User, UserStatus}},
};
use pulsar_db::repo::users;

use crate::{middleware::auth::AuthUser, state::AppState};

pub async fn get_me(auth: AuthUser, State(state): State<AppState>) -> Result<Json<User>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let row = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(Json(User {
        id: Snowflake(row.id),
        username: row.username,
        email: row.email,
        password_hash: row.password_hash,
        avatar_url: row.avatar_url,
        status: UserStatus::default(),
    }))
}
