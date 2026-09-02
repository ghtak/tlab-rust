mod app_config;

use app_config::AppConfig;

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let config = AppConfig::load()?;

    tlab::tracing::initialize(&config.tracing)?;
    tracing::info!("{:#?}", config);
    Ok(())
}
