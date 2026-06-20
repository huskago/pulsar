use axum::{routing::get, Json, Router};
use pulsar_auth::jwt::JwtManager;
use serde::Serialize;
use pulsar_db::pool::{self, DatabaseConfig};
use pulsar_messaging::nats_client::{NatsClient, NatsConfig};
use pulsar_scylla::ScyllaClient;
use pulsar_storage::{StorageClient, StorageConfig};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod connection;
mod crypto_helpers;
mod handler;
mod redis_client;
mod state;

use connection::ConnectionManager;
use state::GatewayState;

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
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    assert!(jwt_secret.len() >= 32, "JWT_SECRET must be at least 32 characters");
    let jwt = JwtManager::new(&jwt_secret);

    let db_config = DatabaseConfig::from_env();
    let db = pool::create_pool(&db_config)
        .await
        .expect("Failed to connect to PostgreSQL");

    let nats_config = NatsConfig::from_env();
    let nats = NatsClient::connect(&nats_config)
        .await
        .expect("Failed to connect to NATS");

    let crypto = pulsar_crypto::CryptoManager::from_env()
        .expect("Invalid KEK_SECRET, must be 64 hex chars (32 bytes)");

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis = redis_client::create_pool(&redis_url)
        .expect("Failed to create Redis pool");

    let scylla_url = std::env::var("SCYLLA_URL")
        .unwrap_or_else(|_| "localhost:9042".to_string());
    let scylla = ScyllaClient::connect(&scylla_url)
        .await
        .expect("Failed to connect to ScyllaDB");

    let storage = StorageClient::new(StorageConfig::from_env())
        .await
        .expect("Failed to connect to MinIO");

    let state = GatewayState {
        connections: ConnectionManager::new(),
        jwt,
        nats,
        db,
        redis,
        crypto,
        scylla,
        storage,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/gateway", get(handler::ws_upgrade))
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3001);
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", addr, e));

    info!("⚡ Pulsar Gateway listening on {}", addr);

    axum::serve(listener, app).await
        .expect("Gateway server failed");
}
