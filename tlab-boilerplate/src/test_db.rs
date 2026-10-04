use std::sync::Arc;

use crate::app_container::AppDB;

const DATABASE_URL: &str = "postgres://tlab-test:tlab-test@localhost:35432/tlab-test";

pub async fn connect() -> Arc<AppDB> {
    Arc::new(
        AppDB::new(&tlab::sqlxdb::Config {
            url: DATABASE_URL.into(),
            max_connections: 1,
        })
        .await
        .unwrap(),
    )
}

pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@example.com", uuid::Uuid::new_v4())
}
