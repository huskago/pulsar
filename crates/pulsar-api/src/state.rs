use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::LiveKitConfig;
use pulsar_storage::StorageClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt: JwtManager,
    pub livekit: LiveKitConfig,
    pub storage: StorageClient,
}
