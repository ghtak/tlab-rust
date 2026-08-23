pub fn map_error(e: sqlx::Error) -> crate::Error {
    crate::Error::DatabaseError(anyhow::Error::new(e))
}

#[allow(dead_code)]
pub trait DatabaseErrorExt {
    fn is_unique_violation(&self) -> bool;
}

impl DatabaseErrorExt for crate::Error {
    fn is_unique_violation(&self) -> bool {
        match self {
            crate::Error::DatabaseError(err) => match err.downcast_ref::<sqlx::Error>() {
                Some(sqlx::Error::Database(db_err)) => match db_err.code().as_deref() {
                    Some("23505") => true,
                    Some("23000") if db_err.message().contains("Duplicate entry") => true,
                    Some("2067") if db_err.message().contains("UNIQUE constraint failed") => true,
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }
}
