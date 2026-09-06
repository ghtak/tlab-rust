mod app_config;
mod app_container;
use std::sync::Arc;

use app_config::AppConfig;
use axum::{handler::HandlerWithoutStateExt, http::StatusCode};

use crate::app_container::AppContainer;

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let config = AppConfig::load()?;

    tlab::tracing::initialize(&config.tracing)?;

    let container = Arc::new(AppContainer::new(config));

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello, world!" }))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(tlab::http::traceparent::new_http_request_span),
        )
        .fallback_service(handle_404.into_service())
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
