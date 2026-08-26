use crate::db::Error;

pub(crate) fn map_error(error: sqlx::Error) -> Error {
    let is_unique_violation = matches!(
        &error,
        sqlx::Error::Database(database_error) if is_unique_violation(database_error.as_ref())
    );
    let source = anyhow::Error::new(error);

    if is_unique_violation {
        Error::UniqueViolation { source }
    } else {
        Error::Driver { source }
    }
}

fn is_unique_violation(error: &(dyn sqlx::error::DatabaseError + 'static)) -> bool {
    match error.code().as_deref() {
        Some("23505") => true,
        Some("1062") | Some("23000") if error.message().contains("Duplicate entry") => true,
        Some("2067") | Some("1555") if error.message().contains("UNIQUE constraint failed") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_non_database_sqlx_errors_to_driver_errors() {
        let error = map_error(sqlx::Error::PoolTimedOut);

        assert!(matches!(error, Error::Driver { .. }));
        assert!(std::error::Error::source(&error).is_some());
    }
}
