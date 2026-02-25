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
