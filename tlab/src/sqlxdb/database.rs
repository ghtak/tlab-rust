use crate::sqlxdb::{self, Session};

#[derive(Debug)]
pub struct Database<DB: sqlx::Database> {
    pub inner: sqlx::Pool<DB>,
}

impl<DB: sqlx::Database> Database<DB> {
    pub async fn new(config: &sqlxdb::Config) -> crate::Result<Self> {
        let inner = sqlx::pool::PoolOptions::<DB>::new()
            .max_connections(config.max_connections)
            .connect(&config.url)
            .await
            .map_err(sqlxdb::map_error)?;
        Ok(Self { inner })
    }

    pub fn session(&self) -> Session<'_, DB> {
        Session::Pool(self.inner.clone())
    }

    pub async fn connection(&self) -> crate::Result<Session<'_, DB>> {
        let conn = self.inner.acquire().await.map_err(sqlxdb::map_error)?;
        Ok(Session::Conn(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlxdb::Config;

    type TestDbSession<'a> = Session<'a, sqlx::Sqlite>;

    async fn create_users_table(session: &mut TestDbSession<'_>) {
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)")
            .execute(session.backend())
            .await
            .unwrap();
    }

    async fn runs_crud(session: &mut TestDbSession<'_>) {
        sqlx::query("INSERT INTO users (name) VALUES (?)")
            .bind("alice")
            .execute(session.backend())
            .await
            .unwrap();

        let name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
            .bind(1_i64)
            .fetch_one(session.backend())
            .await
            .unwrap();
        assert_eq!(name, "alice");

        let error = sqlx::query("INSERT INTO users (name) VALUES (?)")
            .bind("alice")
            .execute(session.backend())
            .await
            .unwrap_err();
        assert!(matches!(
            crate::sqlxdb::map_error(error),
            crate::db::Error::UniqueViolation { .. }
        ));

        sqlx::query("UPDATE users SET name = ? WHERE id = ?")
            .bind("bob")
            .bind(1_i64)
            .execute(session.backend())
            .await
            .unwrap();

        let name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
            .bind(1_i64)
            .fetch_one(session.backend())
            .await
            .unwrap();
        assert_eq!(name, "bob");

        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(1_i64)
            .execute(session.backend())
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(session.backend())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn runs_ddl_and_crud_with_sqlite_in_memory() {
        let database = Database::<sqlx::Sqlite>::new(&Config {
            url: "sqlite::memory:".into(),
            max_connections: 1,
        })
        .await
        .unwrap();
        let mut session = database.session();
        let mut tx = session.begin().await.unwrap();
        create_users_table(&mut tx).await;
        runs_crud(&mut tx).await;
    }
}
