use axum::{routing::get, Router};
use pulsar_auth::jwt::JwtManager;
use pulsar_messaging::nats_client::{NatsClient, NatsConfig};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod connection;
mod handler;
mod state;

use connection::ConnectionManager;
use state::GatewayState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let jwt = JwtManager::new("pulsar-dev-secret");

    let nats_config = NatsConfig::default();
    let nats = NatsClient::connect(&nats_config)
        .await
        .expect("Failed to connect to NATS");

    let state = GatewayState {
        connections: ConnectionManager::new(),
        jwt,
        nats,
    };

    let app = Router::new()
        .route("/gateway", get(handler::ws_upgrade))
        .with_state(state);

    let addr = "0.0.0.0:3001";
    let listener = TcpListener::bind(addr).await.unwrap();

    info!("⚡ Pulsar Gateway listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
