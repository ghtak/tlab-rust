use std::borrow::Cow;

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

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Conflict {0}")]
    Conflict(Cow<'static, str>),

    #[error("{0} not found")]
    NotFound(Cow<'static, str>),

    #[error("Illegal state {0}")]
    IllegalState(Cow<'static, str>),
}

impl Error {
    pub fn not_found(resource: impl Into<Cow<'static, str>>) -> Self {
        Self::NotFound(resource.into())
    }
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
    fn not_found_error_has_resource_message() {
        let err = Error::not_found("user account");
        assert_eq!(err.to_string(), "user account not found");
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
