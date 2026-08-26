use crate::db::Error;

pub(crate) fn map_error(error: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Driver {
        source: anyhow::Error::new(error),
    }
}
