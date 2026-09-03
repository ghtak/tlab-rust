mod app_config;
mod app_container;
use std::sync::Arc;

use app_config::AppConfig;

use crate::app_container::AppContainer;

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let config = AppConfig::load()?;

    tlab::tracing::initialize(&config.tracing)?;

    let container = Arc::new(AppContainer::new(config));

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello, world!" }))
        .with_state(container.clone());

    if let Some(tls_certificate_files) = container.config.tls_certificate_files.as_ref() {
        if !tls_certificate_files.exists() {
            tracing::info!("Generating self-signed certificate");
            tls_certificate_files
                .generate_self_signed_certificate(&vec![container.config.http.host.clone()])?;
        }
        container.http.run_https(app, tls_certificate_files).await?;
    } else {
        container.http.run_http(app).await?;
    }

    Ok(())
}
