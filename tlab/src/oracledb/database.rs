use crate::oracledb::{self, Session};

#[derive(Clone)]
pub struct Database {
    pub inner: deadpool_oracle::Pool,
}

impl Database {
    pub async fn new(config: &oracledb::Config) -> crate::Result<Self> {
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

        let _ = inner.get().await.map_err(oracledb::map_error)?;

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

    fn config() -> Config {
        Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        }
    }

    async fn create_users_table(session: &mut Session<'_>, users_table: &str) {
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
    }

    async fn runs_crud(session: &mut Session<'_>, users_table: &str) {
        session
            .backend()
            .execute(
                &format!("INSERT INTO {users_table} (id, name) VALUES (:1, :2)"),
                &[1_i64.into(), "alice".into()],
            )
            .await
            .unwrap();

        let result = session
            .backend()
            .query(
                &format!("SELECT name FROM {users_table} WHERE id = :1"),
                &[1_i64.into()],
            )
            .await
            .unwrap();
        assert_eq!(result.rows[0].get_string(0), Some("alice"));

        session
            .backend()
            .execute(
                &format!("UPDATE {users_table} SET name = :1 WHERE id = :2"),
                &["bob".into(), 1_i64.into()],
            )
            .await
            .unwrap();

        let result = session
            .backend()
            .query(
                &format!("SELECT name FROM {users_table} WHERE id = :1"),
                &[1_i64.into()],
            )
            .await
            .unwrap();
        assert_eq!(result.rows[0].get_string(0), Some("bob"));

        session
            .backend()
            .execute(
                &format!("DELETE FROM {users_table} WHERE id = :1"),
                &[1_i64.into()],
            )
            .await
            .unwrap();

        let result = session
            .backend()
            .query(&format!("SELECT id FROM {users_table}"), &[])
            .await
            .unwrap();
        assert!(result.rows.is_empty());
    }

    /// cargo test -p tlab --all-features reads_count_with_oracle -- --ignored
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn reads_count_with_oracle() {
        let users_table = users_table();
        let database = Database::new(&config()).await.unwrap();
        let count = {
            let mut session = database.session().await.unwrap();
            create_users_table(&mut session, &users_table).await;

            let result = session
                .backend()
                .query(&format!("SELECT COUNT(*) FROM {users_table}"), &[])
                .await
                .unwrap();
            let count = result.rows[0]
                .get_string(0)
                .unwrap()
                .parse::<i64>()
                .unwrap();

            session
                .backend()
                .execute(&format!("DROP TABLE {users_table} PURGE"), &[])
                .await
                .unwrap();

            count
        };

        assert_eq!(count, 0);
    }

    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn connects_while_building_a_database_pool() {
        let database = Database::new(&config()).await.unwrap();

        assert_eq!(database.inner.status().max_size, 1);
    }

    /// cargo test -p tlab --all-features runs_ddl_and_crud_with_oracle -- --ignored
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn runs_ddl_and_crud_with_oracle() {
        let users_table = users_table();
        let database = Database::new(&config()).await.unwrap();
        let mut session = database.session().await.unwrap();

        create_users_table(&mut session, &users_table).await;

        let mut tx = session.begin().await.unwrap();
        runs_crud(&mut tx, &users_table).await;
        tx.commit().await.unwrap();

        session
            .backend()
            .execute(&format!("DROP TABLE {users_table} PURGE"), &[])
            .await
            .unwrap();
    }

    /// cargo test -p tlab --all-features commits_the_root_transaction_and_rolls_back_a_nested_savepoint -- --ignored
    #[tokio::test]
    #[ignore = "requires tests/docker-db-env Oracle service"]
    async fn commits_the_root_transaction_and_rolls_back_a_nested_savepoint() {
        let users_table = users_table();
        let database = Database::new(&config()).await.unwrap();
        let mut session = database.session().await.unwrap();

        create_users_table(&mut session, &users_table).await;

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
