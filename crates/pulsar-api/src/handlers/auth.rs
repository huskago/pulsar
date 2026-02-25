use axum::{extract::State, Json};
use pulsar_auth::{password};
use pulsar_common::{
    error::AppError,
    models::{
        snowflake::Snowflake,
        user::{AuthResponse, CreateUser, LoginRequest, User, UserStatus},
    },
};
use tracing::info;

use crate::state::AppState;

// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<AuthResponse>, AppError> {
    // 1. Basic validation
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

    // 2. Check that the email address and username are not already taken
    if state.store.email_exists(&payload.email).await {
        return Err(AppError::BadRequest("Email already registered".into()));
    }

    if state.store.username_exists(&payload.username).await {
        return Err(AppError::BadRequest("Username already taken".into()));
    }

    // 3. Hash the password
    let password_hash = password::hash_password(payload.password).await?;

    // 4. Create the user
    // TODO: true Snowflake generator - for now, naive timestamp
    let user_id = Snowflake(chrono::Utc::now().timestamp_millis());

    let user = User {
        id: user_id,
        username: payload.username,
        email: payload.email,
        password_hash,
        avatar_url: None,
        status: UserStatus::default(),
    };

    state.store.insert_user(user.clone()).await;

    // 5. Generate the JWT
    let token = state
        .jwt
        .generate_token(&user_id.to_string(), &user.username)?;

    info!(user_id = %user.id, "New user registered");

    Ok(Json(AuthResponse { token, user }))
}

// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 1. Find the user by email
    let user = state
        .store
        .get_user_by_email(&payload.email)
        .await
        .ok_or(AppError::Unauthorized)?;

    // 2. Verify the password
    let valid = password::verify_password(
        payload.password,
        user.password_hash.clone(),
    )
    .await?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    // 3. Generate the JWT
    let token = state
        .jwt
        .generate_token(&user.id.to_string(), &user.username)?;

    info!(user_id = %user.id, "User logged in");

    Ok(Json(AuthResponse { token, user }))
}