//! Redis/Valkey-backed string cache.

use deadpool_redis::{
    Pool, Runtime,
    redis::{AsyncCommands, AsyncConnectionConfig, cmd},
};
use std::time::Duration;

fn map_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::Error {
    crate::Error::DbDriver {
        source: anyhow::Error::new(error),
    }
}

/// Redis or Valkey connection-pool settings.
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Redis-compatible connection URL.
    pub url: String,
    /// Maximum number of connections held by the pool.
    pub max_connections: usize,
    /// Maximum time to establish a new cache connection, in milliseconds.
    pub connect_timeout_ms: u64,
    /// Maximum time to wait for a cache command response, in milliseconds.
    pub response_timeout_ms: u64,
    /// Maximum time to wait for an available pooled connection, in milliseconds.
    pub pool_wait_timeout_ms: u64,
}

/// A pool-backed Redis/Valkey string cache.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
}

impl Database {
    /// Create the connection pool and verify that the cache server is reachable.
    pub async fn new(config: &Config) -> crate::Result<Self> {
        let connection_config = AsyncConnectionConfig::new()
            .set_connection_timeout(Some(Duration::from_millis(config.connect_timeout_ms)))
            .set_response_timeout(Some(Duration::from_millis(config.response_timeout_ms)));
        let manager =
            deadpool_redis::Manager::new_with_config(config.url.as_str(), connection_config)
                .map_err(map_error)?;
        let pool = Pool::builder(manager)
            .max_size(config.max_connections)
            .wait_timeout(Some(Duration::from_millis(config.pool_wait_timeout_ms)))
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(map_error)?;

        let mut connection = pool.get().await.map_err(map_error)?;
        cmd("PING")
            .query_async::<String>(&mut connection)
            .await
            .map_err(map_error)?;

        Ok(Self { pool })
    }

    /// Get a string value, returning `None` when the key is absent or expired.
    pub async fn get_str(&self, key: &str) -> crate::Result<Option<String>> {
        let mut connection = self.pool.get().await.map_err(map_error)?;
        connection.get(key).await.map_err(map_error)
    }

    /// Store a string value with a required millisecond TTL.
    pub async fn set_str(&self, key: &str, value: &str, ttl_ms: u64) -> crate::Result<()> {
        let mut connection = self.pool.get().await.map_err(map_error)?;
        connection
            .pset_ex(key, value, ttl_ms)
            .await
            .map_err(map_error)
    }

    /// Delete a key and report whether it existed.
    pub async fn delete(&self, key: &str) -> crate::Result<bool> {
        let mut connection = self.pool.get().await.map_err(map_error)?;
        let deleted: usize = connection.del(key).await.map_err(map_error)?;
        Ok(deleted != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tokio::{net::TcpListener, time::sleep};

    async fn valkey_database() -> Database {
        Database::new(&Config {
            url: "redis://localhost:16379".into(),
            max_connections: 1,
            connect_timeout_ms: 100,
            response_timeout_ms: 100,
            pool_wait_timeout_ms: 20,
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Valkey service"]
    async fn stores_reads_and_deletes_a_string() {
        let database = valkey_database().await;
        let key = "tlab:cachedb:test";

        database.set_str(key, "value", 1_000).await.unwrap();
        assert_eq!(
            database.get_str(key).await.unwrap().as_deref(),
            Some("value")
        );
        assert!(database.delete(key).await.unwrap());
        assert_eq!(database.get_str(key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn rejects_an_unavailable_cache_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);

        let result = Database::new(&Config {
            url: format!("redis://{address}"),
            max_connections: 1,
            connect_timeout_ms: 20,
            response_timeout_ms: 20,
            pool_wait_timeout_ms: 20,
        })
        .await;

        assert!(matches!(result, Err(crate::Error::DbDriver { .. })));
    }

    #[tokio::test]
    async fn times_out_when_the_cache_does_not_respond() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let _connection = listener.accept().await.unwrap();
            sleep(Duration::from_secs(1)).await;
        });
        let started = Instant::now();

        let result = Database::new(&Config {
            url: format!("redis://{address}"),
            max_connections: 1,
            connect_timeout_ms: 20,
            response_timeout_ms: 20,
            pool_wait_timeout_ms: 20,
        })
        .await;

        assert!(matches!(result, Err(crate::Error::DbDriver { .. })));
        assert!(started.elapsed() < Duration::from_millis(500));
        server.abort();
    }
}
