#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unique constraint violation")]
    UniqueViolation {
        #[source]
        source: anyhow::Error,
    },

    #[error("database operation failed")]
    Driver {
        #[source]
        source: anyhow::Error,
    },
}
