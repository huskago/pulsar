use pulsar_auth::jwt::JwtManager;
use crate::store::memory::MemoryStore;

#[derive(Clone)]
pub struct AppState {
    pub store: MemoryStore,
    pub jwt: JwtManager,
}