use axum::{routing::{get, post}, Json, Router};
use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::AppConfig;
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;
mod state;
mod store;

use handlers::auth;
use state::AppState;
use store::memory::MemoryStore;


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

    let state = AppState {
        store: MemoryStore::new(),
        jwt,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("🌟 Pulsar API listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
