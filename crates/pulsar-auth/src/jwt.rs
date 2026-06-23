use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pulsar_common::error::AppError;

const ISSUER: &str = "pulsar";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub session_id: String,
    pub jti: String,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_ttl: Duration,
}
impl JwtManager {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_ttl: Duration::minutes(15),
        }
    }

    pub fn generate_token(
        &self,
        user_id: &str,
        username: &str,
        session_id: Uuid,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            session_id: session_id.to_string(),
            jti: Uuid::new_v4().to_string(),
            iss: ISSUER.to_string(),
            exp: (now + self.access_token_ttl).timestamp(),
            iat: now.timestamp(),
        };

        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {}", e)))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["sub", "exp", "iat", "jti", "session_id"]);
        validation.set_issuer(&[ISSUER]);

        let token_data: TokenData<Claims> =
            jsonwebtoken::decode(token, &self.decoding_key, &validation)
                .map_err(|_| AppError::Unauthorized)?;

        Ok(token_data.claims)
    }
}
