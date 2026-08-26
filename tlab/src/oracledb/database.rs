use crate::oracledb::{self, Session};

#[derive(Clone)]
pub struct Database {
    pub inner: deadpool_oracle::Pool,
}

impl Database {
    pub fn new(config: &oracledb::Config) -> crate::Result<Self> {
        let oracle_config = oracle_rs::Config::new(
            &config.host,
            config.port,
            &config.service,
            &config.username,
            &config.password,
        );
        let inner = deadpool_oracle::PoolBuilder::new(oracle_config)
            .max_size(config.max_connections)
            .build()
            .map_err(oracledb::map_error)?;

        Ok(Self { inner })
    }

    pub async fn session(&self) -> crate::Result<Session<'static>> {
        Ok(Session::new(
            self.inner.get().await.map_err(oracledb::map_error)?,
        ))
    }

    pub async fn connection(&self) -> crate::Result<Session<'static>> {
        self.session().await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::oracledb::Config;

    fn users_table() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000_000;
        format!("TLAB_TEST_{}_{timestamp}", std::process::id())
    }

    #[test]
    fn builds_a_database_pool() {
        let database = Database::new(&Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "system".into(),
            password: "password".into(),
            max_connections: 1,
        })
        .unwrap();

        assert_eq!(database.inner.status().max_size, 1);
    }

    /// cargo test -p tlab --all-features commits_the_root_transaction_and_rolls_back_a_nested_savepoint -- --ignored
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn commits_the_root_transaction_and_rolls_back_a_nested_savepoint() {
        let users_table = users_table();
        let database = Database::new(&Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        })
        .unwrap();
        let mut session = database.session().await.unwrap();

        session
            .backend()
            .execute(
                &format!(
                    "CREATE TABLE {users_table} (id NUMBER PRIMARY KEY, name VARCHAR2(64) NOT NULL)"
                ),
                &[],
            )
            .await
            .unwrap();

        let mut root = session.begin().await.unwrap();
        root.backend()
            .execute(
                &format!("INSERT INTO {users_table} (id, name) VALUES (1, 'alice')"),
                &[],
            )
            .await
            .unwrap();

        let mut nested = root.begin().await.unwrap();
        nested
            .backend()
            .execute(
                &format!("UPDATE {users_table} SET name = 'bob' WHERE id = 1"),
                &[],
            )
            .await
            .unwrap();
        nested.rollback().await.unwrap();
        root.commit().await.unwrap();

        let mut root = session.begin().await.unwrap();
        let result = root
            .backend()
            .query(&format!("SELECT name FROM {users_table} WHERE id = 1"), &[])
            .await
            .unwrap();
        assert_eq!(result.rows[0].get_string(0), Some("alice"));
        root.rollback().await.unwrap();

        session
            .backend()
            .execute(&format!("DROP TABLE {users_table} PURGE"), &[])
            .await
            .unwrap();
    }
}
