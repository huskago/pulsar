use axum::{routing::{get, post}, Json, Router};
use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::AppConfig;
use pulsar_db::pool::{self, DatabaseConfig};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;
mod middleware;
mod state;
mod store;

use handlers::{auth, users};
use state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::default();
    let jwt = JwtManager::new("pulsar-dev-secret");

    let db_config = DatabaseConfig::default();
    let db = pool::create_pool(&db_config)
        .await
        .expect("Failed to connect to PostgreSQL");

    let state = AppState { db, jwt };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Public routes
        .route("/health", get(health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        // Protected routes
        .route("/users/me", get(users::get_me))
        .with_state(state)
        .layer(cors);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("🌟 Pulsar API listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
