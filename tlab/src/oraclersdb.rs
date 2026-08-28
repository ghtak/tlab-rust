//! Async Oracle connection-pool wrapper built on `oracle-rs` and `deadpool-oracle`.
//!
//! Use [`Database::conn`] for direct statements or [`Database::tx`] for a root
//! transaction. Obtain [`Context`] and pass [`Context::backend`] to the async
//! `oracle_rs::Connection` APIs.

use oracle_rs::Connection;

pub(crate) fn map_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::Error {
    crate::Error::DbDriver {
        source: anyhow::Error::new(error),
    }
}

/// Oracle connection-pool settings passed to [`Database::new`].
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Oracle listener host name.
    pub host: String,
    /// Oracle listener port.
    pub port: u16,
    /// Oracle service name, such as `FREEPDB1`.
    pub service: String,
    /// Database user name.
    pub username: String,
    /// Database user password.
    pub password: String,
    /// Maximum number of connections held by the pool.
    pub max_connections: usize,
}

/// A `deadpool-oracle` connection pool.
#[derive(Clone)]
pub struct Database {
    pool: deadpool_oracle::Pool,
}

impl Database {
    /// Build the pool and verify that at least one connection can be acquired.
    pub async fn new(config: &Config) -> crate::Result<Self> {
        let oracle_config = oracle_rs::Config::new(
            &config.host,
            config.port,
            &config.service,
            &config.username,
            &config.password,
        );
        let pool = deadpool_oracle::PoolBuilder::new(oracle_config)
            .max_size(config.max_connections)
            .build()
            .map_err(map_error)?;
        let _ = pool.get().await.map_err(map_error)?;
        Ok(Self { pool })
    }

    /// Check out one connection from the pool.
    pub async fn conn(&self) -> crate::Result<Conn> {
        Ok(Conn {
            inner: self.pool.get().await.map_err(map_error)?,
        })
    }

    /// Check out one connection and use it as a root transaction.
    pub async fn tx(&self) -> crate::Result<Tx<'static>> {
        Ok(Tx::root(self.pool.get().await.map_err(map_error)?))
    }
}

/// A checked-out Oracle connection.
pub struct Conn {
    inner: deadpool_oracle::Object,
}

impl Conn {
    /// Borrow the Oracle connection for direct async statements.
    pub fn context(&self) -> Context<'_> {
        Context(&self.inner)
    }
    /// Treat this connection as a root transaction.
    pub fn begin(&self) -> Tx<'_> {
        Tx::borrowed(&self.inner)
    }
}

enum TxInner<'a> {
    Owned(deadpool_oracle::Object),
    Borrowed(&'a Connection),
}

/// A root transaction or nested savepoint.
pub struct Tx<'a> {
    inner: TxInner<'a>,
    savepoint: Option<String>,
    depth: u32,
}

impl Tx<'static> {
    fn root(inner: deadpool_oracle::Object) -> Self {
        Self {
            inner: TxInner::Owned(inner),
            savepoint: None,
            depth: 0,
        }
    }
}

impl<'a> Tx<'a> {
    fn borrowed(inner: &'a Connection) -> Self {
        Self {
            inner: TxInner::Borrowed(inner),
            savepoint: None,
            depth: 0,
        }
    }

    /// Borrow the transaction connection for statement execution.
    pub fn context(&self) -> Context<'_> {
        Context(self.connection())
    }

    /// Create a nested transaction backed by an Oracle savepoint.
    pub async fn begin(&self) -> crate::Result<Tx<'_>> {
        let depth = self.depth + 1;
        let savepoint = format!("tlab_sp_{depth}");
        self.connection()
            .savepoint(&savepoint)
            .await
            .map_err(map_error)?;
        Ok(Tx {
            inner: TxInner::Borrowed(self.connection()),
            savepoint: Some(savepoint),
            depth,
        })
    }

    /// Commit a root transaction. Nested transactions are released implicitly.
    pub async fn commit(self) -> crate::Result<()> {
        if self.savepoint.is_none() {
            self.connection().commit().await.map_err(map_error)?;
        }
        Ok(())
    }

    /// Roll back a root transaction or roll back to a nested savepoint.
    pub async fn rollback(self) -> crate::Result<()> {
        match &self.savepoint {
            Some(savepoint) => self
                .connection()
                .rollback_to_savepoint(savepoint)
                .await
                .map_err(map_error)?,
            None => self.connection().rollback().await.map_err(map_error)?,
        }
        Ok(())
    }
    fn connection(&self) -> &Connection {
        match &self.inner {
            TxInner::Owned(connection) => connection,
            TxInner::Borrowed(connection) => connection,
        }
    }
}

/// A short-lived view of the async `oracle-rs` connection.
pub struct Context<'a>(&'a Connection);

impl Context<'_> {
    /// Return the connection for `oracle-rs` async query and execute calls.
    pub fn backend(&self) -> &Connection {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oracle_rs::Value;

    fn test_config() -> Config {
        Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        }
    }

    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn runs_crud_with_oracle() {
        let database = Database::new(&test_config()).await.unwrap();
        let tx = database.tx().await.unwrap();
        {
            let context = tx.context();
            let connection = context.backend();
            connection
                .execute(
                    "BEGIN EXECUTE IMMEDIATE 'DROP TABLE tlab_test_users PURGE'; EXCEPTION WHEN OTHERS THEN IF SQLCODE != -942 THEN RAISE; END IF; END;",
                    &[],
                )
                .await
                .unwrap();
            connection
            .execute(
                "CREATE TABLE tlab_test_users (id NUMBER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY, name VARCHAR2(100) NOT NULL)",
                &[],
            )
            .await
            .unwrap();
            connection
                .execute(
                    "INSERT INTO tlab_test_users (name) VALUES (:1)",
                    &[Value::String("alice".into())],
                )
                .await
                .unwrap();
            let result = connection
                .query(
                    "SELECT name FROM tlab_test_users WHERE id = :1",
                    &[Value::Integer(1)],
                )
                .await
                .unwrap();
            assert_eq!(result.rows[0].get_string(0), Some("alice"));
            connection
                .execute(
                    "UPDATE tlab_test_users SET name = :1 WHERE id = :2",
                    &[Value::String("bob".into()), Value::Integer(1)],
                )
                .await
                .unwrap();
            let result = connection
                .query(
                    "SELECT name FROM tlab_test_users WHERE id = :1",
                    &[Value::Integer(1)],
                )
                .await
                .unwrap();
            assert_eq!(result.rows[0].get_string(0), Some("bob"));
            connection
                .execute(
                    "DELETE FROM tlab_test_users WHERE id = :1",
                    &[Value::Integer(1)],
                )
                .await
                .unwrap();
            let result = connection
                .query(
                    "SELECT CASE WHEN COUNT(*) = 0 THEN 'empty' ELSE 'not empty' END FROM tlab_test_users",
                    &[],
                )
                .await
                .unwrap();
            assert_eq!(result.rows[0].get_string(0), Some("empty"));
        }
        tx.commit().await.unwrap();
        database
            .conn()
            .await
            .unwrap()
            .context()
            .backend()
            .execute("DROP TABLE tlab_test_users PURGE", &[])
            .await
            .unwrap();
    }
}
