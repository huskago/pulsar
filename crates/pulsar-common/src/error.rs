use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal server error")]
    Internal(#[source] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: u16,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            AppError::Internal(e) => tracing::error!(error = %e, "Internal error"),
            other => tracing::warn!(error = %other),
        }

        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".into()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong".into(),
            ),
        };

        let body = ErrorBody {
            error: message,
            code: status.as_u16(),
        };

        (status, Json(body)).into_response()
    }
}
