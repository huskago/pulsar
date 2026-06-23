use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use pulsar_auth::password;
use pulsar_common::{
    error::AppError,
    models::{
        snowflake::Snowflake,
        user::{AuthResponse, CreateUser, LoginRequest, SelfUserResponse, UserStatus},
    },
};
use pulsar_db::repo::{sessions, users};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::info;
use uuid::Uuid;

use crate::{middleware::auth::AuthUser, state::AppState};

fn hash_token(token: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().to_vec()
}

fn refresh_cookie(token: &str, max_age_secs: i64) -> Cookie<'static> {
    Cookie::build(("refresh_token", token.to_owned()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::seconds(max_age_secs))
        .path("/")
        .build()
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<CreateUser>,
) -> Result<(CookieJar, Json<AuthResponse>), AppError> {
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

    let user_id = Snowflake::generate().0;

    let row = users::insert(
        &state.db,
        user_id,
        &payload.username,
        &payload.email,
        &password_hash,
    )
    .await?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::days(30);

    let session_id = sessions::insert(
        &state.db,
        row.id,
        "Unknown device",
        &token_hash,
        expires_at,
    )
    .await?;

    let (token, jti) = state
        .jwt
        .generate_token(&row.id.to_string(), &row.username, session_id)?;
    let _ = sessions::update_jti(&state.db, session_id, &jti).await;

    info!(user_id = %row.id, "New user registered");

    let updated_jar = jar.add(refresh_cookie(&refresh_token, 30 * 86400));
    Ok((updated_jar, Json(AuthResponse {
        token,
        user: SelfUserResponse {
            id: row.id.to_string(),
            username: row.username,
            email: row.email,
            avatar_url: row.avatar_url,
            status: UserStatus::from_db(&row.status),
        },
    })))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), AppError> {
    let row = users::find_by_email(&state.db, &payload.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let valid = password::verify_password(payload.password, row.password_hash.clone()).await?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    let device_name = payload
        .device_name
        .unwrap_or_else(|| "Unknown device".to_string());
    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::days(30);

    let session_id = sessions::insert(
        &state.db,
        row.id,
        &device_name,
        &token_hash,
        expires_at,
    )
    .await?;

    let (token, jti) = state
        .jwt
        .generate_token(&row.id.to_string(), &row.username, session_id)?;
    let _ = sessions::update_jti(&state.db, session_id, &jti).await;

    info!(user_id = %row.id, "User logged in");

    let updated_jar = jar.add(refresh_cookie(&refresh_token, 30 * 86400));
    Ok((updated_jar, Json(AuthResponse {
        token,
        user: SelfUserResponse {
            id: row.id.to_string(),
            username: row.username,
            email: row.email,
            avatar_url: row.avatar_url,
            status: UserStatus::from_db(&row.status),
        },
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<LoginResponse>), AppError> {
    let token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let token_hash = hash_token(&token);
    let session = sessions::find_by_token_hash(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let user = users::find_by_id(&state.db, session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let new_refresh_token = Uuid::new_v4().to_string();
    let new_hash = hash_token(&new_refresh_token);
    sessions::update_token_hash(&state.db, session.id, &new_hash).await?;

    let (access_token, jti) =
        state
            .jwt
            .generate_token(&session.user_id.to_string(), &user.username, session.id)?;
    let _ = sessions::update_jti(&state.db, session.id, &jti).await;

    let updated_jar = jar
        .remove(Cookie::build("refresh_token").path("/").build())
        .add(refresh_cookie(&new_refresh_token, 30 * 86400));
    Ok((updated_jar, Json(LoginResponse { access_token })))
}

pub async fn logout(
    auth: AuthUser,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AppError> {
    let session_id: Uuid = auth
        .claims
        .session_id
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    sessions::delete(&state.db, session_id).await?;

    let now = Utc::now().timestamp();
    let ttl = (auth.claims.exp - now).max(1) as u64;
    crate::redis_client::block_jwt(&state.redis, &auth.claims.jti, ttl)
        .await
        .unwrap_or_else(|e| tracing::warn!("Failed to block JWT in Redis: {}", e));

    let updated_jar = jar.remove(Cookie::build("refresh_token").path("/").build());
    Ok((updated_jar, StatusCode::NO_CONTENT))
}

pub async fn list_sessions(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<sessions::SessionRow>>, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let session_list = sessions::list_for_user(&state.db, user_id).await?;
    Ok(Json(session_list))
}

pub async fn revoke_session(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let (deleted, jti) = sessions::delete_owned_returning_jti(&state.db, session_id, user_id).await?;
    if !deleted {
        return Err(AppError::Forbidden);
    }
    if let Some(j) = jti {
        crate::redis_client::block_jwt(&state.redis, &j, 15 * 60)
            .await
            .unwrap_or_else(|e| tracing::warn!("Failed to block JWT on session revoke: {}", e));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_all_sessions(
    auth: AuthUser,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AppError> {
    let user_id: i64 = auth.claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

    let jtis = sessions::list_jtis_for_user(&state.db, user_id).await.unwrap_or_default();
    for jti in jtis {
        crate::redis_client::block_jwt(&state.redis, &jti, 15 * 60)
            .await
            .unwrap_or_else(|e| tracing::warn!("Failed to block JWT in Redis: {}", e));
    }

    sessions::delete_all_for_user(&state.db, user_id).await?;

    let updated_jar = jar.remove(Cookie::build("refresh_token").path("/").build());
    Ok((updated_jar, StatusCode::NO_CONTENT))
}
