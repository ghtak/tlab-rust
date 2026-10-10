use std::sync::Arc;

use axum::{extract::State, http::header, routing::get};
use serde::Serialize;

use crate::{
    api_response::ApiResponse, app_container::AppContainer, auth::access_claims::AccessClaims,
    metrics::SystemSnapshot,
};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new().route("/api/v1/dashboard/snapshot", get(snapshot))
}

#[derive(Serialize)]
struct ApplicationSnapshot {
    http_requests_in_flight: u64,
    database_pool: tlab::sqlxdb::PoolStatus,
}

#[derive(Serialize)]
struct SnapshotResponse {
    #[serde(flatten)]
    system: SystemSnapshot,
    application: ApplicationSnapshot,
}

async fn snapshot(
    State(container): State<Arc<AppContainer>>,
    _claims: AccessClaims,
) -> Result<impl axum::response::IntoResponse, ApiResponse<()>> {
    let system = container
        .metrics
        .latest()
        .await
        .ok_or_else(|| ApiResponse::service_unavailable("system metrics are not ready"))?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        ApiResponse::data(SnapshotResponse {
            system,
            application: ApplicationSnapshot {
                http_requests_in_flight: container.metrics.requests_in_flight().saturating_sub(1),
                database_pool: container.database.pool_status(),
            },
        }),
    ))
}
