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

        Ok(AuthUser { claims })
    }
}
