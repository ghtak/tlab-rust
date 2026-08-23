mod database;
mod error;
mod session;

pub use database::SqlxDatabase;
pub use error::*;
pub use session::SqlxSession;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub url: String,
    pub max_connections: u32,
}
