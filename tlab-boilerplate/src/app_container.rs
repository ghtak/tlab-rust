use std::sync::Arc;

use tlab::Result;

use super::app_config::AppConfig;
use crate::metrics::Metrics;

pub type AppDB = tlab::sqlxdb::Database<sqlx::Postgres>;
pub type AppDBCtx<'a> = tlab::sqlxdb::Context<'a, sqlx::Postgres>;
pub type AppQueryBuilder = sqlx::QueryBuilder<sqlx::Postgres>;

pub struct AppContainer {
    pub config: AppConfig,
    pub http: tlab::http::Server,
    pub database: Arc<AppDB>,
    pub metrics: Arc<Metrics>,
    pub password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
    pub jwt_codec: Arc<tlab::jwt::JwtCodec>,
    pub rbac_service: Arc<crate::auth::service::RbacService>,
    pub token_service: Arc<crate::auth::service::TokenService>,
}

impl AppContainer {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let database = Arc::new(AppDB::new(&config.database).await?);
        let password_hasher = Arc::new(tlab::hash::Argon2PasswordHasher::new(
            &config.password_hash,
        )?);
        let jwt_codec = Arc::new(tlab::jwt::JwtCodec::new(&config.jwt)?);
        let token_service = Arc::new(crate::auth::service::TokenService::new(jwt_codec.clone()));
        Ok(Self {
            config: config.clone(),
            http: tlab::http::Server::new(config.http.clone()),
            database: database.clone(),
            metrics: Arc::new(Metrics::new()),
            password_hasher,
            jwt_codec,
            rbac_service: Arc::new(crate::auth::service::RbacService::new(database.clone())),
            token_service,
        })
    }
}
