use std::sync::Arc;

use crate::{app_container::AppContainer, app_response::AppResponse};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new().route("/sample", axum::routing::get(sample_handler))
}

async fn sample_handler() -> AppResponse<String> {
    AppResponse::data("Hello, world!".to_string())
}
