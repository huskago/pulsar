use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use pulsar_common::error::AppError;
use pulsar_db::repo::channels;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::{middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Deserialize)]
pub struct VoiceTokenRequest {
    pub channel_id: String,
}

#[derive(Debug, Serialize)]
pub struct VoiceTokenResponse {
    pub token: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
struct LiveKitClaims {
    iss: String,
    sub: String,
    nbf: u64,
    exp: u64,
    name: String,
    video: VideoGrants,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoGrants {
    room_join: bool,
    room: String,
    can_publish: bool,
    can_subscribe: bool,
    can_publish_data: bool,
}

// POST /voice/token
pub async fn get_voice_token(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<VoiceTokenRequest>,
) -> Result<Json<VoiceTokenResponse>, AppError> {
    let user_id: i64 = auth
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized)?;

    let channel_id: i64 = payload
        .channel_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid channel ID".into()))?;

    let channel = channels::find_by_id(&state.db, channel_id)
        .await?
        .ok_or(AppError::NotFound("Channel not found".into()))?;

    if channel.kind != "voice" {
        return Err(AppError::BadRequest("Not a voice channel".into()));
    }

    let room_name = format!("{}_{}", channel.guild_id, channel_id);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = LiveKitClaims {
        iss: state.livekit.api_key.clone(),
        sub: user_id.to_string(),
        nbf: now,
        exp: now + 3600,
        name: auth.claims.username.clone(),
        video: VideoGrants {
            room_join: true,
            room: room_name,
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
        },
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.livekit.api_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Token generation failed: {}", e)))?;

    info!(
        user_id = %user_id,
        channel_id = %channel_id,
        "Voice token generated"
    );

    Ok(Json(VoiceTokenResponse {
        token,
        url: state.livekit.url.clone(),
    }))
}