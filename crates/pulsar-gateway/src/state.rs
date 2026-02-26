use crate::connection::ConnectionManager;
use pulsar_auth::jwt::JwtManager;

#[derive(Clone)]
pub struct GatewayState {
    pub connections: ConnectionManager,
    pub jwt: JwtManager,
}
