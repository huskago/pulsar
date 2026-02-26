use pulsar_auth::jwt::JwtManager;
use pulsar_messaging::nats_client::NatsClient;
use sqlx::PgPool;
use crate::connection::ConnectionManager;

#[derive(Clone)]
pub struct GatewayState {
    pub connections: ConnectionManager,
    pub jwt: JwtManager,
    pub nats: NatsClient,
    pub db: PgPool,
}