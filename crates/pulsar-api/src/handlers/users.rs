use crate::{middleware::auth::AuthUser, state::AppState};
use axum::{extract::State, Json};
use pulsar_common::{
    error::AppError,
    models::{snowflake::Snowflake, user::User},
};

pub async fn get_me(auth: AuthUser, State(state): State<AppState>) -> Result<Json<User>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let user = state
        .store
        .get_user_by_id(Snowflake(user_id))
        .await
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(Json(user))
}
