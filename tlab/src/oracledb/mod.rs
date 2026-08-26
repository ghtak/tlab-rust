mod database;
mod error;
mod session;

pub use database::Database;
pub(crate) use error::map_error;
pub use session::Session;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub service: String,
    pub username: String,
    pub password: String,
    pub max_connections: usize,
}
