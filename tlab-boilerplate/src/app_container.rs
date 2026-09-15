use std::sync::Arc;

use tlab::Result;

use super::app_config::AppConfig;

pub type AppDB = tlab::sqlxdb::Database<sqlx::Postgres>;
pub type AppDBCtx<'a> = tlab::sqlxdb::Context<'a, sqlx::Postgres>;

pub struct AppContainer {
    pub config: AppConfig,
    pub http: tlab::http::Server,
    pub database: Arc<AppDB>,
}

impl AppContainer {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let database = Arc::new(AppDB::new(&config.database).await?);
        Ok(Self {
            config: config.clone(),
            http: tlab::http::Server::new(config.http.clone()),
            database,
        })
    }
}
