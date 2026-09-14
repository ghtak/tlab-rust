use tlab::Result;

use super::app_config::AppConfig;

pub struct AppContainer {
    pub config: AppConfig,
    pub http: tlab::http::Server,
    pub database: tlab::sqlxdb::Database<sqlx::Postgres>,
}

impl AppContainer {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let database = tlab::sqlxdb::Database::<sqlx::Postgres>::new(&config.database).await?;
        Ok(Self {
            config: config.clone(),
            http: tlab::http::Server::new(config.http.clone()),
            database,
        })
    }
}
