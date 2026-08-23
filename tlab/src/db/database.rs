use crate::db::{self, SqlxSession};

#[derive(Debug)]
pub struct SqlxDatabase<DB: sqlx::Database> {
    pub inner: sqlx::Pool<DB>,
}

impl<DB: sqlx::Database> SqlxDatabase<DB> {
    pub async fn new(config: &db::Config) -> crate::Result<Self> {
        let inner = sqlx::pool::PoolOptions::<DB>::new()
            .max_connections(config.max_connections)
            .connect(&config.url)
            .await
            .map_err(db::map_error)?;
        Ok(Self { inner })
    }

    pub fn pool(&self) -> SqlxSession<'_, DB>
    where
        for<'e> &'e mut <DB as sqlx::Database>::Connection: sqlx::Executor<'e, Database = DB>,
    {
        SqlxSession::Pool(self.inner.clone())
    }

    pub async fn tx(&self) -> crate::Result<SqlxSession<'_, DB>>
    where
        for<'e> &'e mut <DB as sqlx::Database>::Connection: sqlx::Executor<'e, Database = DB>,
    {
        let tx = self.inner.begin().await.map_err(db::map_error)?;
        Ok(SqlxSession::Tx(tx))
    }

    pub async fn conn(&self) -> crate::Result<SqlxSession<'_, DB>>
    where
        for<'e> &'e mut <DB as sqlx::Database>::Connection: sqlx::Executor<'e, Database = DB>,
    {
        let conn = self.inner.acquire().await.map_err(db::map_error)?;
        Ok(SqlxSession::Conn(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Config;

    type TestDbSession<'a> = SqlxSession<'a, sqlx::Sqlite>;

    async fn create_users_table(session: &mut TestDbSession<'_>) {
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
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
        let database = SqlxDatabase::<sqlx::Sqlite>::new(&Config {
            url: "sqlite::memory:".into(),
            max_connections: 1,
        })
        .await
        .unwrap();
        let mut session = database.pool();

        create_users_table(&mut session).await;
        runs_crud(&mut session).await;
    }
}
