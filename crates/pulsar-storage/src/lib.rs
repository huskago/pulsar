use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use md5::{Digest, Md5};

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:9000".to_string(),
            bucket: "pulsar-uploads".to_string(),
            access_key: "pulsar".to_string(),
            secret_key: "pulsarsecret".to_string(),
        }
    }
}

impl StorageConfig {
    pub fn from_env() -> Self {
        let endpoint = std::env::var("STORAGE_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".to_string());
        let bucket = std::env::var("STORAGE_BUCKET")
            .unwrap_or_else(|_| "pulsar-uploads".to_string());
        Self {
            endpoint,
            bucket,
            access_key: std::env::var("STORAGE_ACCESS_KEY")
                .unwrap_or_else(|_| "pulsar".to_string()),
            secret_key: std::env::var("STORAGE_SECRET_KEY")
                .unwrap_or_else(|_| "pulsarsecret".to_string()),
        }
    }
}

#[derive(Clone)]
pub struct StorageClient {
    client: Client,
    bucket: String,
}

pub struct UploadResult {
    pub key: String,
    pub size: u64,
}

impl StorageClient {
    pub async fn new(config: StorageConfig) -> anyhow::Result<Self> {
        let creds = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "pulsar-storage",
        );

        let s3_config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(&config.endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);

        let buckets = client.list_buckets().send().await?;
        let exists = buckets
            .buckets()
            .iter()
            .any(|b| b.name().unwrap_or_default() == config.bucket);

        if !exists {
            client.create_bucket().bucket(&config.bucket).send().await?;
            info!(bucket = %config.bucket, "Created storage bucket");
        }

        let _ = client.delete_bucket_policy().bucket(&config.bucket).send().await;
        info!(bucket = %config.bucket, "Bucket set to private (no public policy)");

        Ok(Self { client, bucket: config.bucket })
    }

    pub async fn upload(
        &self,
        channel_id: &str,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> anyhow::Result<UploadResult> {
        let size = data.len() as u64;
        let ext = filename.rsplit('.').next().unwrap_or("bin");
        let key = format!("{}/{}.{}", channel_id, Uuid::new_v4(), ext);

        let mut hasher = Md5::new();
        Digest::update(&mut hasher, &data);
        let content_md5 = STANDARD.encode(hasher.finalize());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(content_type)
            .content_md5(&content_md5)
            .body(ByteStream::from(data))
            .send()
            .await?;

        info!(key = %key, size = %size, "File uploaded (encrypted)");
        Ok(UploadResult { key, size })
    }

    pub async fn generate_presigned_url(&self, key: &str, expiry_secs: u64) -> anyhow::Result<String> {
        let config = PresigningConfig::expires_in(Duration::from_secs(expiry_secs))
            .map_err(|e| anyhow::anyhow!("Presigning config error: {}", e))?;
        let presigned = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| anyhow::anyhow!("Presign failed for key {}: {}", key, e))?;
        Ok(presigned.uri().to_string())
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }
}
