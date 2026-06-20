use crate::state::GatewayState;
use pulsar_crypto::Dek;

pub async fn get_channel_dek(state: &GatewayState, channel_id: i64) -> anyhow::Result<Dek> {
    if let Ok(Some(sealed)) = crate::redis_client::get_sealed_dek(&state.redis, channel_id).await {
        return state.crypto.open_dek(&sealed);
    }

    let sealed = pulsar_db::repo::channel_keys::find(&state.db, channel_id)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("No DEK for channel {}", channel_id))?;

    let _ = crate::redis_client::set_sealed_dek(&state.redis, channel_id, &sealed).await;

    state.crypto.open_dek(&sealed)
}
