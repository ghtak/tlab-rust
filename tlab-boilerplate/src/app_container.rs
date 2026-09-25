use std::sync::Arc;

use tlab::Result;

use super::app_config::AppConfig;

pub type AppDB = tlab::sqlxdb::Database<sqlx::Postgres>;
pub type AppDBCtx<'a> = tlab::sqlxdb::Context<'a, sqlx::Postgres>;

pub struct AppContainer {
    pub config: AppConfig,
    pub http: tlab::http::Server,
    pub database: Arc<AppDB>,
    pub password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
    pub jwt_codec: Arc<tlab::jwt::JwtCodec>,
}

impl AppContainer {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let database = Arc::new(AppDB::new(&config.database).await?);
        let password_hasher = Arc::new(tlab::hash::Argon2PasswordHasher::new(
            &config.password_hash,
        )?);
        let jwt_codec = Arc::new(tlab::jwt::JwtCodec::new(&config.jwt)?);
        Ok(Self {
            config: config.clone(),
            http: tlab::http::Server::new(config.http.clone()),
            database,
            password_hasher,
            jwt_codec,
        })
    }
}
