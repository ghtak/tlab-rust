use std::sync::Arc;

use axum::{extract::State, http::header, response::IntoResponse, routing::post};

use crate::{
    app_container::AppContainer,
    app_response::AppResponse,
    auth::{
        self,
        usecase::{CreateManagedUserCommand, LoginManagedUserCommand},
    },
};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth/user", post(create_managed_user))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
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
    let command = CreateManagedUserCommand {
        name: request.name,
        email: request.email,
        password: request.password,
    };

    let usecase = auth::usecase::CreateManagedUserUsecase::new(
        container.database.clone(),
        container.password_hasher.clone(),
    );

    let resp = usecase.execute(&command).await;

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

#[derive(Debug, Clone, serde::Deserialize)]
struct LoginManagedUserRequest {
    email: String,
    password: String,
}

#[derive(serde::Serialize)]
struct LoginManagedUserResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
}

async fn login(
    State(container): State<Arc<AppContainer>>,
    axum::Json(request): axum::Json<LoginManagedUserRequest>,
) -> impl IntoResponse {
    let command = LoginManagedUserCommand {
        email: request.email,
        password: request.password,
    };
    let usecase = auth::usecase::LoginManagedUserUsecase::new(
        container.database.clone(),
        container.password_hasher.clone(),
        container.jwt_codec.clone(),
    );

    let response = match usecase.execute(&command).await {
        Ok(result) => AppResponse::data(LoginManagedUserResponse {
            access_token: result.tokens.access.token,
            refresh_token: result.tokens.refresh.token,
            token_type: "Bearer",
            expires_in: container.config.jwt.access_token_ttl_seconds,
        }),
        Err(tlab::Error::InvalidCredentials) => AppResponse::unauthorized("invalid credentials"),
        Err(error) => {
            tracing::error!(?error, "Failed to log in managed user");
            AppResponse::internal_error("failed to log in")
        }
    };

    ([(header::CACHE_CONTROL, "no-store")], response)
}

async fn logout(State(_container): State<Arc<AppContainer>>) -> AppResponse<()> {
    AppResponse::ok()
}
