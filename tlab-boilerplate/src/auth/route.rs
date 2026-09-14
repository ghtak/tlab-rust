use std::sync::Arc;

use crate::app_container::AppContainer;

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new().route("/auth", axum::routing::get(auth_handler))
}

async fn auth_handler() -> &'static str {
    "Auth route"
}