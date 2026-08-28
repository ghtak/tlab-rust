use crate::{oracledb, oraclersdb};
use std::time::{Duration, Instant};

const WARMUP_ITERATIONS: usize = 20;
const ITERATIONS: usize = 100;

fn oracle_configs() -> (oraclersdb::Config, oracledb::Config) {
    (
        oraclersdb::Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        },
        oracledb::Config {
            host: "localhost".into(),
            port: 11521,
            service: "FREEPDB1".into(),
            username: "tlab".into(),
            password: "Tlab_123".into(),
            max_connections: 1,
        },
    )
}

async fn measure_oraclersdb(config: &oraclersdb::Config) -> Duration {
    let database = oraclersdb::Database::new(config).await.unwrap();
    let connection = database.conn().await.unwrap();
    let context = connection.context();

    for _ in 0..WARMUP_ITERATIONS {
        context
            .backend()
            .query("SELECT 1 FROM DUAL", &[])
            .await
            .unwrap();
    }

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        context
            .backend()
            .query("SELECT 1 FROM DUAL", &[])
            .await
            .unwrap();
    }
    started.elapsed()
}

async fn measure_oracledb(config: oracledb::Config) -> Duration {
    let database = oracledb::Database::new(config).unwrap();
    let context = database.conn().await.unwrap().context();

    for _ in 0..WARMUP_ITERATIONS {
        context
            .run_blocking(|connection| {
                connection
                    .query_row("SELECT 1 FROM DUAL", &[])
                    .map(|_| ())
                    .map_err(oracledb::map_error)
            })
            .await
            .unwrap();
    }

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        context
            .run_blocking(|connection| {
                connection
                    .query_row("SELECT 1 FROM DUAL", &[])
                    .map(|_| ())
                    .map_err(oracledb::map_error)
            })
            .await
            .unwrap();
    }
    started.elapsed()
}

/// Compare single-connection Oracle query latency after a short warm-up.
///
/// Run with `cargo test -p tlab measures_oracle_query_latency -- --ignored --nocapture`.
#[tokio::test]
#[ignore = "requires tests/docker-db-env Oracle service"]
async fn measures_oracle_query_latency() {
    let (oraclersdb_config, oracledb_config) = oracle_configs();
    let oracledb_elapsed = measure_oracledb(oracledb_config).await;
    let oraclersdb_elapsed = measure_oraclersdb(&oraclersdb_config).await;

    println!(
        "oracle-rs: {oraclersdb_elapsed:?} total, {:?} per query",
        oraclersdb_elapsed / ITERATIONS as u32,
    );
    println!(
        "oracledb: {oracledb_elapsed:?} total, {:?} per query",
        oracledb_elapsed / ITERATIONS as u32,
    );
}
