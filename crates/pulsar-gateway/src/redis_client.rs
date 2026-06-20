use deadpool_redis::{Config, Pool, Runtime};
use deadpool_redis::redis::AsyncCommands;

pub fn create_pool(redis_url: &str) -> anyhow::Result<Pool> {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .map_err(|e| anyhow::anyhow!("Redis pool creation failed: {}", e))
}

pub async fn get_sealed_dek(pool: &Pool, channel_id: i64) -> anyhow::Result<Option<Vec<u8>>> {
    let mut conn = pool.get().await
        .map_err(|e| anyhow::anyhow!("Redis connection failed: {}", e))?;
    let key = format!("dek:{}", channel_id);
    let result: Option<Vec<u8>> = conn.get(&key).await
        .map_err(|e| anyhow::anyhow!("Redis GET failed: {}", e))?;
    Ok(result)
}

pub async fn set_sealed_dek(pool: &Pool, channel_id: i64, sealed_dek: &[u8]) -> anyhow::Result<()> {
    let mut conn = pool.get().await
        .map_err(|e| anyhow::anyhow!("Redis connection failed: {}", e))?;
    let key = format!("dek:{}", channel_id);
    let _: () = conn.set_ex(&key, sealed_dek, 300).await
        .map_err(|e| anyhow::anyhow!("Redis SET failed: {}", e))?;
    Ok(())
}
