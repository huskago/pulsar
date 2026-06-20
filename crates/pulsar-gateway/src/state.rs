use crate::connection::ConnectionManager;
use deadpool_redis::Pool as RedisPool;
use pulsar_auth::jwt::JwtManager;
use pulsar_crypto::CryptoManager;
use pulsar_messaging::nats_client::NatsClient;
use pulsar_scylla::ScyllaClient;
use pulsar_storage::StorageClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct GatewayState {
    pub connections: ConnectionManager,
    pub jwt: JwtManager,
    pub nats: NatsClient,
    pub db: PgPool,
    pub redis: RedisPool,
    pub crypto: CryptoManager,
    pub scylla: ScyllaClient,
    pub storage: StorageClient,
}
