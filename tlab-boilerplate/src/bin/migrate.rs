#[path = "../migration_sql.rs"]
mod migration_sql;

use std::path::PathBuf;

use serde::Deserialize;
use tlab::config::Loader;

#[derive(Deserialize)]
struct Config {
    database: tlab::sqlxdb::Config,
}

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.yml");
    let config = Loader::from_file(config_path).try_deserialize::<Config>()?;
    let database = tlab::sqlxdb::Database::<sqlx::Postgres>::new(&config.database).await?;
    let mut tx = database.tx().await?;

    for (name, sql) in migration_sql::AUTH_MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(tx.context().backend())
            .await
            .map_err(|source| tlab::Error::DbDriver {
                source: source.into(),
            })?;
        println!("Migration applied: {name}");
    }

    tx.commit().await?;
    Ok(())
}
