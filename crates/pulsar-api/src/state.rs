use deadpool_redis::Pool as RedisPool;
use pulsar_auth::jwt::JwtManager;
use pulsar_common::config::LiveKitConfig;
use pulsar_crypto::CryptoManager;
use pulsar_scylla::ScyllaClient;
use pulsar_storage::StorageClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt: JwtManager,
    pub livekit: LiveKitConfig,
    pub storage: StorageClient,
    pub storage_endpoint: String,
    pub redis: RedisPool,
    pub crypto: CryptoManager,
    pub scylla: ScyllaClient,
}
