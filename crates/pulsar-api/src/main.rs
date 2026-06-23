use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue};
use axum::{
    routing::{get, post, patch, put, delete}, Json,
    Router,
};
use std::net::SocketAddr;
use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::{AppConfig, LiveKitConfig};
use pulsar_db::pool::{self, DatabaseConfig};
use pulsar_storage::{StorageClient, StorageConfig};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;
mod middleware;
mod redis_client;
mod state;

use handlers::{auth, users, guilds, channels, invites, voice, uploads, roles, dms, relationships};
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
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    {
        let lk_secret = std::env::var("LIVEKIT_API_SECRET").unwrap_or_default();
        assert!(lk_secret.len() >= 32, "LIVEKIT_API_SECRET must be at least 32 characters");
    }
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    assert!(jwt_secret.len() >= 32, "JWT_SECRET must be at least 32 characters");
    let jwt = JwtManager::new(&jwt_secret);

    let db_config = DatabaseConfig::from_env();
    let db = pool::create_pool(&db_config)
        .await
        .expect("Failed to connect to PostgreSQL");

    let storage = StorageClient::new(StorageConfig::from_env())
        .await
        .expect("Failed to connect to MinIO");

    let crypto = pulsar_crypto::CryptoManager::from_env()
        .expect("Invalid KEK_SECRET, must be 64 hex chars (32 bytes)");

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis = redis_client::create_pool(&redis_url)
        .expect("Failed to create Redis pool");

    let scylla_url = std::env::var("SCYLLA_URL")
        .unwrap_or_else(|_| "localhost:9042".to_string());
    let scylla = pulsar_scylla::ScyllaClient::connect(&scylla_url)
        .await
        .expect("Failed to connect to ScyllaDB");

    let auth_limiter = middleware::rate_limit::AuthRateLimiter::new();
    let invite_limiter = middleware::rate_limit::AuthRateLimiter::new();

    let storage_endpoint = std::env::var("STORAGE_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9000".to_string());

    let state = AppState {
        db,
        jwt,
        livekit: LiveKitConfig::from_env(),
        storage,
        storage_endpoint,
        redis,
        crypto,
        scylla,
    };

    let allowed_origins: Vec<HeaderValue> = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .layer(axum::middleware::from_fn(
            move |req, next| {
                let limiter = auth_limiter.clone();
                middleware::rate_limit::auth_rate_limit_fn(limiter, req, next)
            }
        ));

    let invite_routes = Router::new()
        .route("/invites/{code}", get(invites::get_invite))
        .route("/invites/{code}/join", post(invites::join_invite))
        .layer(axum::middleware::from_fn(
            move |req, next| {
                let limiter = invite_limiter.clone();
                middleware::rate_limit::auth_rate_limit_fn(limiter, req, next)
            }
        ));

    let app = Router::new()
        .route("/health", get(health))
        .merge(auth_routes)
        .merge(invite_routes)
        .route("/auth/logout", post(auth::logout))
        .route("/auth/sessions", get(auth::list_sessions).delete(auth::revoke_all_sessions))
        .route("/auth/sessions/{id}", delete(auth::revoke_session))
        .route("/users/me", get(users::get_me).patch(users::update_me))
        .route("/users/me/settings",
               get(users::get_settings).patch(users::update_settings))
        .route(
            "/guilds",
            get(guilds::list_guilds).post(guilds::create_guild),
        )
        .route(
            "/guilds/{guild_id}",
            patch(guilds::update_guild).delete(guilds::delete_guild),
        )
        .route(
            "/guilds/{guild_id}/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/channels/{channel_id}",
            patch(channels::update_channel).delete(channels::delete_channel),
        )
        .route(
            "/channels/{channel_id}/messages",
            get(channels::list_messages),
        )
        .route(
            "/channels/{channel_id}/messages/{message_id}",
            patch(channels::edit_message).delete(channels::delete_message),
        )
        .route(
            "/guilds/{guild_id}/invites",
            get(invites::list_invites).post(invites::create_invite),
        )
        .route("/voice/token", post(voice::get_voice_token))
        .route("/upload", post(uploads::upload_file)
            .layer(DefaultBodyLimit::max(26 * 1024 * 1024)))
        .route("/attachments/{key}/url", get(uploads::get_attachment_url))
        .route("/guilds/{guild_id}/roles",
               get(roles::list_roles).post(roles::create_role))
        .route("/guilds/{guild_id}/roles/{role_id}",
               patch(roles::update_role).delete(roles::delete_role))
        .route("/guilds/{guild_id}/roles/{role_id}/members",
               put(roles::assign_role))
        .route("/guilds/{guild_id}/roles/{role_id}/members/{user_id}",
               delete(roles::unassign_role))
        .route("/relationships",
               get(relationships::list_relationships).post(relationships::create_relationship))
        .route("/relationships/{user_id}",
               put(relationships::update_relationship).delete(relationships::delete_relationship))
        .route("/relationships/{user_id}/mutual-friends",
               get(relationships::get_mutual_friends))
        .route("/dms", get(dms::list_dms).post(dms::open_dm))
        .route("/dms/group", post(dms::create_group_dm))
        .with_state(state)
        .layer(cors);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", addr, e));

    info!("🌟 Pulsar API listening on {}", addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
        .expect("API server failed");
}
