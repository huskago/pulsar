use crate::store::memory::MemoryStore;
use pulsar_auth::jwt::JwtManager;

#[derive(Clone)]
pub struct AppState {
    pub store: MemoryStore,
    pub jwt: JwtManager,
}
