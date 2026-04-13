use axum::http::header;
use axum::{
    routing::{get, post, patch, put, delete}, Json,
    Router,
};
use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::{AppConfig, LiveKitConfig};
use pulsar_db::pool::{self, DatabaseConfig};
use pulsar_storage::{StorageClient, StorageConfig};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;
mod middleware;
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
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "pulsar-dev-secret".to_string());
    let jwt = JwtManager::new(&jwt_secret);

    let db_config = DatabaseConfig::from_env();
    let db = pool::create_pool(&db_config)
        .await
        .expect("Failed to connect to PostgreSQL");

    let storage = StorageClient::new(StorageConfig::from_env())
        .await
        .expect("Failed to connect to MinIO");

    let state = AppState {
        db,
        jwt,
        livekit: LiveKitConfig::from_env(),
        storage,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        // Public routes
        .route("/health", get(health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/invites/{code}", get(invites::get_invite))
        // Protected routes
        .route("/users/me", get(users::get_me))
        .route("/users/me/settings",
               get(users::get_settings).patch(users::update_settings))
        .route(
            "/guilds",
            get(guilds::list_guilds).post(guilds::create_guild),
        )
        .route(
            "/guilds/{guild_id}/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/channels/{channel_id}/messages",
            get(channels::list_messages),
        )
        .route(
            "/guilds/{guild_id}/invites",
            get(invites::list_invites).post(invites::create_invite),
        )
        .route("/invites/{code}/join", post(invites::join_invite))
        .route("/voice/token", post(voice::get_voice_token))
        .route("/upload", post(uploads::upload_file))
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
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("🌟 Pulsar API listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
