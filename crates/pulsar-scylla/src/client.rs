use std::sync::Arc;
use scylla::{Session, SessionBuilder};
use tracing::info;

#[derive(Clone)]
pub struct ScyllaClient {
    session: Arc<Session>,
}

impl ScyllaClient {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let session = SessionBuilder::new()
            .known_node(url)
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("ScyllaDB connection failed ({}): {}", url, e))?;

        init_schema(&session).await?;
        info!(url = %url, "Connected to ScyllaDB");

        Ok(Self { session: Arc::new(session) })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}

async fn init_schema(session: &Session) -> anyhow::Result<()> {
    session.query_unpaged(
        "CREATE KEYSPACE IF NOT EXISTS pulsar \
         WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}",
        &[] as &[u8; 0],
    ).await?;

    session.query_unpaged(
        "CREATE TABLE IF NOT EXISTS pulsar.messages ( \
            channel_id  BIGINT, \
            message_id  BIGINT, \
            author_id   BIGINT, \
            content     BLOB, \
            edited_at   BIGINT, \
            PRIMARY KEY (channel_id, message_id) \
         ) WITH CLUSTERING ORDER BY (message_id DESC)",
        &[] as &[u8; 0],
    ).await?;

    info!("ScyllaDB schema initialized");
    Ok(())
}
