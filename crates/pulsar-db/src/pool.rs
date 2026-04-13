use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://pulsar:pulsar@localhost:5432/pulsar".to_string(),
            max_connections: 10,
        }
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://pulsar:pulsar@localhost:5432/pulsar".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}

pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await?;

    info!("Connected to PostgreSQL");

    sqlx::migrate!("./migrations").run(&pool).await?;

    info!("Migrations applied");

    Ok(pool)
}
