use super::app_config::AppConfig;

pub struct AppContainer {
    pub config: AppConfig,
    pub http: tlab::http::Server,
}

impl AppContainer {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: config.clone(),
            http: tlab::http::Server::new(config.http.clone()),
        }
    }
}
