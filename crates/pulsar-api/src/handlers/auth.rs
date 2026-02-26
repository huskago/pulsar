use axum::{extract::State, Json};
use pulsar_auth::password;
use pulsar_common::{
    error::AppError,
    models::{
        snowflake::Snowflake,
        user::{AuthResponse, CreateUser, LoginRequest, User, UserStatus},
    },
};
use pulsar_db::repo::users;
use tracing::info;

use crate::state::AppState;

// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<AuthResponse>, AppError> {
    if payload.username.len() < 2 || payload.username.len() > 32 {
        return Err(AppError::BadRequest(
            "Username must be between 2 and 32 characters".into(),
        ));
    }

    if !payload.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email".into()));
    }

    if payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let password_hash = password::hash_password(payload.password).await?;

    let user_id = chrono::Utc::now().timestamp_millis();

    let row = users::insert(
        &state.db,
        user_id,
        &payload.username,
        &payload.email,
        &password_hash,
    )
    .await?;

    let token = state
        .jwt
        .generate_token(&row.id.to_string(), &row.username)?;

    let user = User {
        id: Snowflake(row.id),
        username: row.username,
        email: row.email,
        password_hash: row.password_hash,
        avatar_url: row.avatar_url,
        status: UserStatus::default(),
    };

    info!(user_id = %user.id, "New user registered");

    Ok(Json(AuthResponse { token, user }))
}

// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let row = users::find_by_email(&state.db, &payload.email)
        .await?
        .ok_or(AppError::Unauthorized)?;


    let valid = password::verify_password(
        payload.password,
        row.password_hash.clone()
    )
        .await?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    let token = state
        .jwt
        .generate_token(&row.id.to_string(), &row.username)?;

    let user = User {
        id: Snowflake(row.id),
        username: row.username,
        email: row.email,
        password_hash: row.password_hash,
        avatar_url: row.avatar_url,
        status: UserStatus::default(),
    };

    info!(user_id = %user.id, "User logged in");

    Ok(Json(AuthResponse { token, user }))
}
