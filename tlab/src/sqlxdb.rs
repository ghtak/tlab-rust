mod database;
mod error;
mod session;

pub use database::Database;
pub(crate) use error::map_error;
pub use session::Session;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub url: String,
    pub max_connections: u32,
}
