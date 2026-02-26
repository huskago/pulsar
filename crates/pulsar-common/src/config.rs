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