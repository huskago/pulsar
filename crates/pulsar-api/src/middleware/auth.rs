use axum::{extract::FromRequestParts, http::request::Parts};

use pulsar_auth::jwt::Claims;
use pulsar_common::error::AppError;

use crate::state::AppState;

pub struct AuthUser {
    pub claims: Claims,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = state.jwt.validate_token(token)?;

        match crate::redis_client::is_jwt_blocked(&state.redis, &claims.jti).await {
            Ok(true) => return Err(AppError::Unauthorized),
            Ok(false) => {}
            Err(e) => {
                tracing::error!("Redis blocklist check failed, rejecting request: {}", e);
                return Err(AppError::Unauthorized);
            }
        }

        let session_id_str = claims.session_id.clone();
        let db = state.db.clone();
        tokio::spawn(async move {
            if let Ok(sid) = session_id_str.parse::<uuid::Uuid>() {
                let _ = pulsar_db::repo::sessions::touch(&db, sid).await;
            }
        });

        Ok(AuthUser { claims })
    }
}
