use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use pulsar_common::models::{
    snowflake::Snowflake,
    user::User,
};

#[derive(Clone, Default)]
pub struct MemoryStore {
    users: Arc<RwLock<HashMap<Snowflake, User>>>,
    email_index: Arc<RwLock<HashMap<String, Snowflake>>>,
    username_index: Arc<RwLock<HashMap<String, Snowflake>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert_user(&self, user: User) {
        let id = user.id;
        let email = user.email.clone();
        let username = user.username.clone();

        self.users.write().await.insert(id, user);
        self.email_index.write().await.insert(email, id);
        self.username_index.write().await.insert(username, id);
    }

    pub async fn get_user_by_id(&self, id: Snowflake) -> Option<User> {
        self.users.read().await.get(&id).cloned()
    }

    pub async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let id = self.email_index.read().await.get(email).copied()?;
        self.get_user_by_id(id).await
    }

    pub async fn email_exists(&self, email: &str) -> bool {
        self.email_index.read().await.contains_key(email)
    }

    pub async fn username_exists(&self, username: &str) -> bool {
        self.username_index
            .read()
            .await
            .contains_key(&username.to_lowercase())
    }
}
