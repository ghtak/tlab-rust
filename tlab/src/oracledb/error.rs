use crate::db::Error;

pub(crate) fn map_error(error: impl std::error::Error + Send + Sync + 'static) -> Error {
    let source = anyhow::Error::new(error);

    if matches!(
        source.downcast_ref::<oracle_rs::Error>(),
        Some(oracle_rs::Error::OracleError { code: 1, .. })
    ) {
        Error::UniqueViolation { source }
    } else {
        Error::Driver { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_oracle_unique_constraint_errors_to_unique_violations() {
        let error = map_error(oracle_rs::Error::oracle(1, "unique constraint violated"));

        assert!(matches!(error, Error::UniqueViolation { .. }));
    }
}
