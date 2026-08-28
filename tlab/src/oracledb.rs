//! Oracle connection-pool wrapper for the synchronous `oracledb` driver.
//!
//! Use [`Database::conn`] or [`Database::tx`] from async code. Execute driver
//! calls through [`Context::run_blocking`] so they do not block the Tokio runtime.

use oracledb::{Connection, Pool};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
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
    /// Maximum number of checked-out connections.
    pub max_connections: usize,
}
/// A thread-safe Oracle pool with an async checkout limit.
#[derive(Clone)]
pub struct Database {
    pool: Arc<Pool>,
    permits: Arc<Semaphore>,
}
impl Database {
    /// Create an Oracle pool from the supplied connection settings.
    pub fn new(config: Config) -> crate::Result<Self> {
        let connect_string = format!("{}:{}/{}", config.host, config.port, config.service);
        let pool_config = oracledb::PoolConfig::default()
            .set_credentials(&config.username, &config.password)
            .set_connect_string(&connect_string)
            .map_err(map_error)?
            .set_min_connections(0)
            .set_max_connections(config.max_connections);
        let pool = oracledb::create_pool(pool_config).map_err(map_error)?;
        Ok(Self {
            pool: Arc::new(pool),
            permits: Arc::new(Semaphore::new(config.max_connections)),
        })
    }
    /// Check out one connection without blocking the async runtime.
    pub async fn conn(&self) -> crate::Result<Conn> {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(map_semaphore_error)?;
        let pool = Arc::clone(&self.pool);
        let connection = tokio::task::spawn_blocking(move || pool.acquire())
            .await
            .map_err(map_join_error)?
            .map_err(map_error)?;
        Ok(Conn {
            lease: Arc::new(Lease {
                connection,
                _permit: permit,
            }),
        })
    }
    /// Check out one connection and use it as a root transaction.
    pub async fn tx(&self) -> crate::Result<Tx> {
        Ok(Tx::root(self.conn().await?.lease))
    }
}
struct Lease {
    connection: Connection,
    _permit: OwnedSemaphorePermit,
}
/// A checked-out Oracle connection.
pub struct Conn {
    lease: Arc<Lease>,
}
impl Conn {
    /// Clone a context for running statements on this connection.
    pub fn context(&self) -> Context {
        Context(Arc::clone(&self.lease))
    }
    /// Treat this connection as a root transaction.
    pub fn begin(&self) -> Tx {
        Tx::root(Arc::clone(&self.lease))
    }
}
/// A root transaction or nested savepoint.
pub struct Tx {
    lease: Arc<Lease>,
    savepoint: Option<String>,
    depth: u32,
}
impl Tx {
    fn root(lease: Arc<Lease>) -> Self {
        Self {
            lease,
            savepoint: None,
            depth: 0,
        }
    }
    /// Clone a context for running statements in this transaction.
    pub fn context(&self) -> Context {
        Context(Arc::clone(&self.lease))
    }
    /// Create a nested transaction backed by an Oracle savepoint.
    pub async fn begin(&self) -> crate::Result<Self> {
        let depth = self.depth + 1;
        let savepoint = format!("tlab_sp_{depth}");
        let lease = Arc::clone(&self.lease);
        let savepoint_sql = format!("SAVEPOINT {savepoint}");
        tokio::task::spawn_blocking(move || lease.connection.execute(&savepoint_sql, &[]))
            .await
            .map_err(map_join_error)?
            .map_err(map_error)?;
        Ok(Self {
            lease: Arc::clone(&self.lease),
            savepoint: Some(savepoint),
            depth,
        })
    }
    /// Commit a root transaction. Nested transactions are released implicitly.
    pub async fn commit(self) -> crate::Result<()> {
        if self.savepoint.is_none() {
            tokio::task::spawn_blocking(move || self.connection().commit())
                .await
                .map_err(map_join_error)?
                .map_err(map_error)?;
        }
        Ok(())
    }
    /// Roll back a root transaction or roll back to a nested savepoint.
    pub async fn rollback(self) -> crate::Result<()> {
        if let Some(savepoint) = &self.savepoint {
            let lease = Arc::clone(&self.lease);
            let rollback_sql = format!("ROLLBACK TO SAVEPOINT {savepoint}");
            tokio::task::spawn_blocking(move || lease.connection.execute(&rollback_sql, &[]))
                .await
                .map_err(map_join_error)?
                .map_err(map_error)?;
        } else {
            tokio::task::spawn_blocking(move || self.connection().rollback())
                .await
                .map_err(map_join_error)?
                .map_err(map_error)?;
        }
        Ok(())
    }
    fn connection(&self) -> &Connection {
        &self.lease.connection
    }
}
/// A cloneable view of the checked-out synchronous Oracle connection.
pub struct Context(Arc<Lease>);
impl Context {
    /// Return the synchronous driver connection.
    ///
    /// Prefer [`Self::run_blocking`] from async code.
    pub fn backend(&self) -> &Connection {
        &self.0.connection
    }

    /// Run a synchronous driver operation on Tokio's blocking thread pool.
    ///
    /// The closure receives the checked-out `oracledb::Connection` and its result
    /// should be mapped with this module's database error handling.
    pub async fn run_blocking<T, F>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&Connection) -> crate::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let lease = Arc::clone(&self.0);
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || span.in_scope(|| f(&lease.connection)))
            .await
            .map_err(map_join_error)?
    }
}

pub(crate) fn map_error(error: oracledb::Error) -> crate::Error {
    crate::Error::DbDriver {
        source: anyhow::anyhow!(error.to_string()),
    }
}
fn map_semaphore_error(error: tokio::sync::AcquireError) -> crate::Error {
    crate::Error::DbDriver {
        source: anyhow::Error::new(error),
    }
}
fn map_join_error(error: tokio::task::JoinError) -> crate::Error {
    crate::Error::DbDriver {
        source: anyhow::Error::new(error),
    }
}
#[cfg(test)]
mod tests {

    use super::*;
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn runs_crud_with_oracle() {
        let database = Database::new(Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        })
        .unwrap();
        let tx = database.tx().await.unwrap();
        let context = tx.context();
        context
            .run_blocking(|connection| {
                connection
                    .execute(
                        "CREATE TABLE tlab_test_oracledb_users (id NUMBER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY, name VARCHAR2(100) NOT NULL)",
                        &[],
                    )
                    .map_err(map_error)
            })
            .await
            .unwrap();
        context
            .run_blocking(|connection| {
                connection
                    .execute(
                        "INSERT INTO tlab_test_oracledb_users (name) VALUES (:1)",
                        &[&"alice"],
                    )
                    .map_err(map_error)
            })
            .await
            .unwrap();
        let name: String = context
            .run_blocking(|connection| {
                connection
                    .query_row(
                        "SELECT name FROM tlab_test_oracledb_users WHERE id = :1",
                        &[&1_i64],
                    )
                    .and_then(|row| row.get(0))
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(name, "alice");
        context
            .run_blocking(|connection| {
                connection
                    .execute(
                        "UPDATE tlab_test_oracledb_users SET name = :1 WHERE id = :2",
                        &[&"bob", &1_i64],
                    )
                    .map_err(map_error)
            })
            .await
            .unwrap();
        let name: String = context
            .run_blocking(|connection| {
                connection
                    .query_row(
                        "SELECT name FROM tlab_test_oracledb_users WHERE id = :1",
                        &[&1_i64],
                    )
                    .and_then(|row| row.get(0))
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(name, "bob");
        context
            .run_blocking(|connection| {
                connection
                    .execute(
                        "DELETE FROM tlab_test_oracledb_users WHERE id = :1",
                        &[&1_i64],
                    )
                    .map_err(map_error)
            })
            .await
            .unwrap();
        let count: i64 = context
            .run_blocking(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM tlab_test_oracledb_users", &[])
                    .and_then(|row| row.get(0))
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(count, 0);
        tx.commit().await.unwrap();
        context
            .run_blocking(|connection| {
                connection
                    .execute("DROP TABLE tlab_test_oracledb_users PURGE", &[])
                    .map_err(map_error)
            })
            .await
            .unwrap();
    }
}
