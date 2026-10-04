use std::sync::Arc;

use crate::{
    api_response::{ApiResponse, ApiResult},
    app_container::AppContainer,
    auth::{
        self,
        access_claims::AccessClaims,
        permission,
        usecase::{CreateManagedUserCommand, LoginManagedUserCommand, LogoutUserUsecase},
    },
};
use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
    routing::{get, post},
};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth/user", post(create_managed_user))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
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
    claims: AccessClaims,
    axum::Json(request): axum::Json<CreateManagedUserRequest>,
) -> ApiResult<CreateManagedUserResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let command = CreateManagedUserCommand {
        name: request.name,
        email: request.email,
        password: request.password,
    };

    let usecase = auth::usecase::CreateManagedUserUsecase::new(
        container.database.clone(),
        container.password_hasher.clone(),
    );

    let user = usecase
        .execute(&command)
        .await
        .map_err(|error| match error {
            tlab::Error::Conflict(_) => ApiResponse::conflict("user already exists"),
            error => {
                tracing::error!(?error, "Failed to create managed user");
                ApiResponse::internal_error("failed to create user")
            }
        })?;

    Ok(ApiResponse::created(CreateManagedUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        status: user.status.as_str().to_owned(),
    }))
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
) -> Result<impl IntoResponse, ApiResponse<()>> {
    let command = LoginManagedUserCommand {
        email: request.email,
        password: request.password,
    };
    let usecase = auth::usecase::LoginManagedUserUsecase::new(
        container.database.clone(),
        container.password_hasher.clone(),
        container.jwt_codec.clone(),
    );

    let result = usecase
        .execute(&command)
        .await
        .map_err(|error| match error {
            tlab::Error::InvalidCredentials => ApiResponse::unauthorized("invalid credentials"),
            error => {
                tracing::error!(?error, "Failed to log in managed user");
                ApiResponse::internal_error("failed to log in")
            }
        })?;

    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        ApiResponse::data(LoginManagedUserResponse {
            access_token: result.tokens.access.token,
            refresh_token: result.tokens.refresh.token,
            token_type: "Bearer",
            expires_in: container.config.jwt.access_token_ttl_seconds,
        }),
    ))
}

async fn logout(State(container): State<Arc<AppContainer>>, claims: AccessClaims) -> ApiResult<()> {
    let user_account_id = claims
        .jwt
        .sub
        .parse::<i64>()
        .map_err(|_| ApiResponse::unauthorized("invalid token"))?;
    LogoutUserUsecase::new(container.database.clone())
        .execute(user_account_id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to log out user");
            ApiResponse::internal_error("failed to log out")
        })?;

    Ok(ApiResponse::ok())
}

async fn me(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
) -> ApiResult<tlab::jwt::JwtClaims> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;
    Ok(ApiResponse::data(claims.jwt))
}
