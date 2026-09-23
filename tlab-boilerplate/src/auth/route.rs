use std::sync::Arc;

use axum::extract::State;

use crate::{
    app_container::AppContainer,
    app_response::AppResponse,
    auth::{self, usecase::CreateManagedUserCommand},
};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth", axum::routing::get(auth_handler))
        .route(
            "/api/v1/auth/user",
            axum::routing::post(create_managed_user),
        )
}

async fn auth_handler() -> &'static str {
    "Auth route"
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateManagedUserRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(serde::Serialize)]
struct CreateManagedUserResponse {
    id: i64,
    name: String,
    email: String,
    status: String,
}

async fn create_managed_user(
    State(container): State<Arc<AppContainer>>,
    axum::Json(request): axum::Json<CreateManagedUserRequest>,
) -> AppResponse<CreateManagedUserResponse> {
    let usecase = auth::usecase::CreateManagedUserUsecase::new(
        container.database.clone(),
        container.password_hasher.clone(),
    );
    let resp = usecase
        .execute(CreateManagedUserCommand {
            name: request.name,
            email: request.email,
            password: request.password,
        })
        .await;

    match resp {
        Ok(user) => AppResponse::created(CreateManagedUserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            status: user.status.as_str().to_owned(),
        }),
        Err(tlab::Error::Conflict(_)) => AppResponse::conflict("user already exists"),
        Err(error) => {
            tracing::error!(?error, "Failed to create managed user");
            AppResponse::internal_error("failed to create user")
        }
    }
}
