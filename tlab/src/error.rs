use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("db driver error: {source}")]
    DbDriver {
        #[source]
        source: anyhow::Error,
    },
}

pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_string() {
        let err = Error::Internal(anyhow::anyhow!("test"));
        assert_eq!(err.to_string(), "internal error: test");
    }

    #[test]
    fn test_context() {
        let err = Error::Internal(anyhow::anyhow!("test").context("context"));
        match err {
            Error::Internal(e) => {
                assert_eq!(format!("{e:#}"), "context: test");
            }
            _ => unreachable!(),
        }
    }
}
