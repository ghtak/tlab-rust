use std::sync::atomic::{AtomicU64, Ordering};

use crate::app_container::AppDB;

static NEXT_SCHEMA_ID: AtomicU64 = AtomicU64::new(0);

pub async fn connect() -> AppDB {
    AppDB::new(&tlab::sqlxdb::Config {
        url: "postgres://tlab:tlab@localhost:25432/tlab".into(),
        max_connections: 1,
    })
    .await
    .unwrap()
}

pub async fn isolated_tx<'a>(
    database: &'a AppDB,
    migrations: &[&'static str],
) -> tlab::sqlxdb::Tx<'a, sqlx::Postgres> {
    let mut tx = database.tx().await.unwrap();
    let schema = format!(
        "tlab_test_{}_{}",
        std::process::id(),
        NEXT_SCHEMA_ID.fetch_add(1, Ordering::Relaxed)
    );

    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(tx.context().backend())
        .await
        .unwrap();
    sqlx::query_scalar::<_, String>("SELECT set_config('search_path', $1, true)")
        .bind(&schema)
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
    for &sql in migrations {
        sqlx::raw_sql(sql)
            .execute(tx.context().backend())
            .await
            .unwrap();
    }

    tx
}
