mod api_response;
mod app_config;
mod app_container;
mod auth;
mod dashboard;
mod metrics;
mod migration;
#[cfg(test)]
mod test_app;
#[cfg(test)]
mod test_db;
use std::sync::Arc;

use app_config::AppConfig;
use axum::{handler::HandlerWithoutStateExt, http::StatusCode};
use clap::Parser;
use tower_http::services::ServeDir;

use crate::app_container::{AppContainer, AppDB};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    migrate: bool,
    #[arg(long)]
    init_admin: bool,
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let args = Args::parse();

    let config = AppConfig::load()?;

    tlab::tracing::initialize(&config.tracing)?;

    if args.migrate {
        let database = AppDB::new(&config.database).await?;
        migration::migrate(&database).await?;
        return Ok(());
    }

    if args.init_admin {
        let database = AppDB::new(&config.database).await?;
        let password_hasher = tlab::hash::Argon2PasswordHasher::new(&config.password_hash)?;
        migration::initialize_admin(&database, &password_hasher).await?;
        return Ok(());
    }

    let container = Arc::new(AppContainer::new(config).await?);

    let app = axum::Router::new()
        .merge(auth::route::router())
        .merge(dashboard::route::router())
        .route_layer(axum::middleware::from_fn_with_state(
            container.clone(),
            auth::cookie::refresh_auth_cookies,
        ));

    let app = if let Some(static_files) = container.config.http.static_files.as_ref() {
        if let Err(e) = static_files.validate() {
            tracing::error!("Invalid static files configuration: {}", e);
            return Err(e);
        }
        app.nest_service(
            &static_files.mount_path,
            ServeDir::new(&static_files.directory),
        )
    } else {
        app
    };

    let app = app
        .fallback_service(handle_404.into_service())
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(tlab::http::trace::new_request_span),
        )
        .layer(axum::middleware::from_fn_with_state(
            container.clone(),
            metrics::track_requests,
        ))
        .with_state(container.clone());

    let metrics_task = metrics::Metrics::start(container.metrics.clone());
    let result =
        if let Some(tls_certificate_files) = container.config.tls_certificate_files.as_ref() {
            tls_certificate_files.ensure_files(&[container.config.http.host.clone()])?;
            container.http.run_https(app, tls_certificate_files).await
        } else {
            container.http.run_http(app).await
        };

    metrics_task.abort();
    let _ = metrics_task.await;

    result
}
