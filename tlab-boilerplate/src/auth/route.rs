use std::sync::Arc;

use crate::app_container::AppContainer;

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth", axum::routing::get(auth_handler))
        .route("/api/v1/auth/user", axum::routing::post(create_user))
}

async fn auth_handler() -> &'static str {
    "Auth route"
}

async fn create_user(
    
) -> &'static str {
    "Create user"
}
