//! SQLx connection-pool wrapper.
//!
//! Create [`Database`] with a database-specific type such as `sqlx::Postgres`,
//! then use [`Database::conn`] for a pooled connection or [`Database::tx`] for
//! a transaction. Pass [`Context::backend`] to SQLx queries.

/// Connection-pool settings passed to [`Database::new`].
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// SQLx connection URL, for example `postgres://user:password@host/database`.
    pub url: String,
    /// Maximum number of connections held by the pool.
    pub max_connections: u32,
}

use crate::sqlxdb::{self};
use sqlx::Acquire;

fn map_error<E: Into<anyhow::Error>>(e: E) -> crate::Error {
    crate::Error::DbDriver { source: e.into() }
}

/// A reusable SQLx connection pool.
#[derive(Debug)]
pub struct Database<DB: sqlx::Database> {
    inner: sqlx::Pool<DB>,
}

impl<DB: sqlx::Database> Database<DB> {
    /// Connect to the database and create a pool from `config`.
    pub async fn new(config: &sqlxdb::Config) -> crate::Result<Self> {
        let inner = sqlx::pool::PoolOptions::<DB>::new()
            .max_connections(config.max_connections)
            .connect(&config.url)
            .await
            .map_err(map_error)?;
        Ok(Self { inner })
    }

    /// Check out one connection from the pool.
    pub async fn conn(&self) -> crate::Result<Conn<DB>> {
        let inner = self.inner.acquire().await.map_err(map_error)?;
        Ok(Conn { inner })
    }

    /// Start a root transaction on a pooled connection.
    pub async fn tx(&self) -> crate::Result<Tx<'_, DB>> {
        let tx = self.inner.begin().await.map_err(map_error)?;
        Ok(Tx { inner: tx })
    }
}

/// A checked-out pooled connection.
///
/// Use [`Self::context`] to execute a query without a transaction, or
/// [`Self::begin`] to start one on this connection.
#[derive(Debug)]
pub struct Conn<DB: sqlx::Database> {
    inner: sqlx::pool::PoolConnection<DB>,
}

impl<DB: sqlx::Database> Conn<DB> {
    /// Start a transaction on this connection.
    pub async fn begin(&mut self) -> crate::Result<Tx<'_, DB>> {
        let inner = self.inner.begin().await.map_err(sqlxdb::map_error)?;
        Ok(Tx { inner })
    }
    /// Borrow the SQLx backend for query execution.
    pub fn context(&mut self) -> Context<'_, DB> {
        Context {
            inner: self.inner.as_mut(),
        }
    }
}

/// A transaction that can be committed, rolled back, or nested with a savepoint.
#[derive(Debug)]
pub struct Tx<'c, DB: sqlx::Database> {
    inner: sqlx::Transaction<'c, DB>,
}

impl<'c, DB: sqlx::Database> Tx<'c, DB> {
    /// Start a nested transaction/savepoint.
    pub async fn begin(&mut self) -> crate::Result<Tx<'_, DB>> {
        let inner = self.inner.begin().await.map_err(sqlxdb::map_error)?;
        Ok(Tx { inner })
    }

    /// Commit this transaction.
    pub async fn commit(self) -> crate::Result<()> {
        self.inner.commit().await.map_err(sqlxdb::map_error)?;
        Ok(())
    }

    /// Roll back this transaction.
    pub async fn rollback(self) -> crate::Result<()> {
        self.inner.rollback().await.map_err(sqlxdb::map_error)?;
        Ok(())
    }

    /// Borrow the transaction's SQLx backend for query execution.
    pub fn context(&mut self) -> Context<'_, DB> {
        Context {
            inner: &mut *self.inner,
        }
    }
}

/// A short-lived wrapper that exposes the active SQLx executor.
#[derive(Debug)]
pub struct Context<'c, DB: sqlx::Database> {
    inner: &'c mut DB::Connection,
}

impl<DB: sqlx::Database> Context<'_, DB> {
    /// Return the SQLx connection to pass to `execute`, `fetch_one`, and similar APIs.
    pub fn backend(&mut self) -> &mut DB::Connection {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestDatabase = Database<sqlx::Sqlite>;
    type TestContext<'c> = Context<'c, sqlx::Sqlite>;
    async fn create_users_table(session: &mut TestContext<'_>) {
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)")
            .execute(session.backend())
            .await
            .unwrap();
    }
    async fn runs_crud(session: &mut TestContext<'_>) {
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
        assert!(matches!(map_error(error), crate::Error::DbDriver { .. }));
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
    fn test_config() -> Config {
        Config {
            url: "sqlite::memory:".into(),
            max_connections: 1,
        }
    }
    async fn test_database() -> TestDatabase {
        TestDatabase::new(&test_config()).await.unwrap()
    }
    async fn postgres_database() -> Database<sqlx::Postgres> {
        Database::new(&Config {
            url: "postgres://tlab:tlab@localhost:15432/tlab".into(),
            max_connections: 1,
        })
        .await
        .unwrap()
    }
    #[tokio::test]
    async fn runs_ddl_and_crud_with_sqlite_in_memory() {
        let database = test_database().await;
        let mut tx = database.tx().await.unwrap();
        create_users_table(&mut tx.context()).await;
        runs_crud(&mut tx.context()).await;
        tx.commit().await.unwrap();
    }
    #[tokio::test]
    async fn connection_provides_a_context_and_can_start_a_transaction() {
        let database = test_database().await;
        let mut conn = database.conn().await.unwrap();
        create_users_table(&mut conn.context()).await;
        let mut tx = conn.begin().await.unwrap();
        runs_crud(&mut tx.context()).await;
        tx.commit().await.unwrap();
    }
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env PostgreSQL service"]
    async fn runs_crud_with_postgres() {
        let database = postgres_database().await;
        {
            let mut conn = database.conn().await.unwrap();
            sqlx::query("DROP TABLE IF EXISTS tlab_test_users")
                .execute(conn.context().backend())
                .await
                .unwrap();
        }
        let mut tx = database.tx().await.unwrap();
        let context = &mut tx.context();
        sqlx::query("CREATE TABLE tlab_test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
            .execute(context.backend())
            .await
            .unwrap();
        sqlx::query("INSERT INTO tlab_test_users (name) VALUES ($1)")
            .bind("alice")
            .execute(context.backend())
            .await
            .unwrap();
        let name: String = sqlx::query_scalar("SELECT name FROM tlab_test_users WHERE id = $1")
            .bind(1_i32)
            .fetch_one(context.backend())
            .await
            .unwrap();
        assert_eq!(name, "alice");
        sqlx::query("UPDATE tlab_test_users SET name = $1 WHERE id = $2")
            .bind("bob")
            .bind(1_i32)
            .execute(context.backend())
            .await
            .unwrap();
        let name: String = sqlx::query_scalar("SELECT name FROM tlab_test_users WHERE id = $1")
            .bind(1_i32)
            .fetch_one(context.backend())
            .await
            .unwrap();
        assert_eq!(name, "bob");
        sqlx::query("DELETE FROM tlab_test_users WHERE id = $1")
            .bind(1_i32)
            .execute(context.backend())
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tlab_test_users")
            .fetch_one(context.backend())
            .await
            .unwrap();
        assert_eq!(count, 0);
        tx.commit().await.unwrap();
        {
            let mut conn = database.conn().await.unwrap();
            sqlx::query("DROP TABLE tlab_test_users")
                .execute(conn.context().backend())
                .await
                .unwrap();
        }
    }
}
