use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveKitConfig {
    pub url: String,
    pub api_key: String,
    pub api_secret: String,
}

impl Default for LiveKitConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:7880".to_string(),
            api_key: "devkey".to_string(),
            api_secret: "secret".to_string(),
        }
    }
}

impl LiveKitConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("LIVEKIT_URL")
                .unwrap_or_else(|_| "ws://localhost:7880".to_string()),
            api_key: std::env::var("LIVEKIT_API_KEY")
                .unwrap_or_else(|_| "devkey".to_string()),
            api_secret: std::env::var("LIVEKIT_API_SECRET")
                .unwrap_or_else(|_| "secret".to_string()),
        }
    }
}
