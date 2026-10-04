use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app_container::AppDB;

static NEXT_SCHEMA_ID: AtomicU64 = AtomicU64::new(0);
const DATABASE_URL: &str = "postgres://tlab:tlab@localhost:25432/tlab";

fn schema_name() -> String {
    format!(
        "tlab_test_{}_{}",
        std::process::id(),
        NEXT_SCHEMA_ID.fetch_add(1, Ordering::Relaxed)
    )
}

pub async fn connect() -> AppDB {
    AppDB::new(&tlab::sqlxdb::Config {
        url: DATABASE_URL.into(),
        max_connections: 1,
    })
    .await
    .unwrap()
}

pub struct IsolatedDb {
    pub database: Arc<AppDB>,
    schema: String,
}

pub async fn isolated_db(migrations: &[&'static str]) -> IsolatedDb {
    let schema = schema_name();
    let database = connect().await;
    let mut conn = database.conn().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(conn.context().backend())
        .await
        .unwrap();
    drop(conn);
    drop(database);

    let database = Arc::new(
        AppDB::new(&tlab::sqlxdb::Config {
            url: format!("{DATABASE_URL}?options[search_path]={schema}"),
            max_connections: 1,
        })
        .await
        .unwrap(),
    );
    let mut conn = database.conn().await.unwrap();
    for &sql in migrations {
        sqlx::raw_sql(sql)
            .execute(conn.context().backend())
            .await
            .unwrap();
    }
    drop(conn);

    IsolatedDb { database, schema }
}

impl IsolatedDb {
    pub async fn cleanup(self) {
        let Self { database, schema } = self;
        drop(database);
        let database = connect().await;
        let mut conn = database.conn().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .execute(conn.context().backend())
            .await
            .unwrap();
    }
}

pub async fn isolated_tx<'a>(
    database: &'a AppDB,
    migrations: &[&'static str],
) -> tlab::sqlxdb::Tx<'a, sqlx::Postgres> {
    let mut tx = database.tx().await.unwrap();
    let schema = schema_name();

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
