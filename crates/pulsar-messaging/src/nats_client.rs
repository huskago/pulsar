use async_nats::{
    jetstream::{self, stream, Context as JetStreamContext},
    Client,
};
use bytes::Bytes;
use pulsar_common::error::AppError;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub stream_name: String,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            stream_name: "PULSAR_CHAT".to_string(),
        }
    }
}

impl NatsConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            stream_name: std::env::var("NATS_STREAM")
                .unwrap_or_else(|_| "PULSAR_CHAT".to_string()),
        }
    }
}

#[derive(Clone)]
pub struct NatsClient {
    client: Client,
    jetstream: JetStreamContext,
}

impl NatsClient {
    pub async fn connect(config: &NatsConfig) -> Result<Self, AppError> {
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("NATS connect failed: {}", e)))?;

        info!(url = %config.url, "Connected to NATS");

        let jetstream = jetstream::new(client.clone());

        jetstream
            .get_or_create_stream(stream::Config {
                name: config.stream_name.clone(),
                subjects: vec![
                    "chat.>".to_string(),
                    "typing.>".to_string(),
                    "presence.>".to_string(),
                ],
                retention: stream::RetentionPolicy::Limits,
                storage: stream::StorageType::File,
                max_bytes: 1_073_741_824,
                max_age: Duration::from_secs(7 * 24 * 60 * 60),
                duplicate_window: Duration::from_secs(120),
                ..Default::default()
            })
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("JetStream stream setup failed: {}", e))
            })?;

        info!(stream = %config.stream_name, "JetStream stream ready");

        Ok(Self { client, jetstream })
    }

    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), AppError> {
        self.client
            .publish(subject.to_string(), Bytes::copy_from_slice(payload))
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("NATS publish failed: {}", e)))?;

        Ok(())
    }

    pub async fn publish_persistent(&self, subject: &str, payload: &[u8]) -> Result<(), AppError> {
        self.jetstream
            .publish(subject.to_string(), Bytes::copy_from_slice(payload))
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JetStream publish failed: {}", e)))?
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JetStream ack failed: {}", e)))?;

        Ok(())
    }

    pub async fn subscribe(&self, subject: &str) -> Result<async_nats::Subscriber, AppError> {
        self.client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("NATS subscribe failed: {}", e)))
    }

    pub fn jetstream(&self) -> &JetStreamContext {
        &self.jetstream
    }
}
