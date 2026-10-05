use std::sync::Arc;

use crate::{
    api_response::{ApiResponse, ApiResult},
    app_container::AppContainer,
    auth::{
        self,
        access_claims::AccessClaims,
        permission,
        usecase::{
            CreateManagedUserCommand, LoginManagedUserCommand, LogoutUserCommand,
            LogoutUserUsecase, RefreshTokenCommand, RefreshTokenUsecase,
        },
    },
};
use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::CookieJar;

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth/user", post(create_managed_user))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh_token))
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

#[derive(Debug, Clone, serde::Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

#[derive(serde::Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
}

async fn login(
    State(container): State<Arc<AppContainer>>,
    jar: CookieJar,
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
        auth::cookie::add_auth_cookies(
            jar,
            &result.tokens.access.token,
            &result.tokens.refresh.token,
            &container,
        ),
        [(header::CACHE_CONTROL, "no-store")],
        ApiResponse::data(TokenResponse {
            access_token: result.tokens.access.token,
            refresh_token: result.tokens.refresh.token,
            token_type: "Bearer",
            expires_in: container.config.jwt.access_token_ttl_seconds,
        }),
    ))
}

async fn refresh_token(
    State(container): State<Arc<AppContainer>>,
    axum::Json(request): axum::Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, ApiResponse<()>> {
    let tokens = RefreshTokenUsecase::new(container.database.clone(), container.jwt_codec.clone())
        .execute(&RefreshTokenCommand {
            refresh_token: request.refresh_token,
        })
        .await
        .map_err(|error| match error {
            tlab::Error::InvalidToken => ApiResponse::unauthorized("invalid token"),
            error => {
                tracing::error!(?error, "Failed to refresh token");
                ApiResponse::internal_error("failed to refresh token")
            }
        })?;

    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        ApiResponse::data(TokenResponse {
            access_token: tokens.access.token,
            refresh_token: tokens.refresh.token,
            token_type: "Bearer",
            expires_in: container.config.jwt.access_token_ttl_seconds,
        }),
    ))
}

async fn logout(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiResponse<()>> {
    let command = LogoutUserCommand {
        user_account_id: claims.user_account_id()?,
        session_id: claims.app.session_id,
    };
    LogoutUserUsecase::new(container.database.clone())
        .execute(&command)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to log out user");
            ApiResponse::internal_error("failed to log out")
        })?;

    Ok((auth::cookie::clear_auth_cookies(jar), ApiResponse::ok()))
}

async fn me(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
) -> ApiResult<tlab::jwt::JwtClaims> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;
    Ok(ApiResponse::data(claims.jwt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_app;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use time::Duration;
    use tlab::jwt::TokenUse;
    use tower::ServiceExt;

    #[tokio::test]
    async fn login_sets_and_logout_clears_auth_cookies() {
        let (app, container) = test_app::setup().await;

        let login = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "email": "admin@localhost",
                            "password": "passwd",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let login_cookies: Vec<_> = login
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| Cookie::parse(value.to_str().unwrap().to_owned()).unwrap())
            .collect();
        assert_eq!(login_cookies.len(), 2);
        for (name, ttl) in [
            (
                "access_token",
                container.config.jwt.access_token_ttl_seconds,
            ),
            (
                "refresh_token",
                container.config.jwt.refresh_token_ttl_seconds,
            ),
        ] {
            let cookie = login_cookies
                .iter()
                .find(|cookie| cookie.name() == name)
                .unwrap();
            assert!(!cookie.value().is_empty());
            assert_eq!(cookie.path(), Some("/"));
            assert_eq!(cookie.max_age(), Some(Duration::seconds(ttl as i64)));
            assert_eq!(cookie.http_only(), Some(true));
            assert_eq!(cookie.secure(), Some(true));
            assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        }

        let access_token = login_cookies
            .iter()
            .find(|cookie| cookie.name() == "access_token")
            .unwrap()
            .value();
        let logout = app
            .oneshot(
                Request::post("/api/v1/auth/logout")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::OK);
        let logout_cookies: Vec<_> = logout
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| Cookie::parse(value.to_str().unwrap().to_owned()).unwrap())
            .collect();
        assert_eq!(logout_cookies.len(), 2);
        for name in ["access_token", "refresh_token"] {
            let cookie = logout_cookies
                .iter()
                .find(|cookie| cookie.name() == name)
                .unwrap();
            assert_eq!(cookie.value(), "");
            assert_eq!(cookie.path(), Some("/"));
            assert_eq!(cookie.max_age(), Some(Duration::ZERO));
        }
    }

    #[tokio::test]
    async fn refresh_rotates_api_token_and_rejects_reuse() {
        let (app, container) = test_app::setup().await;
        let login = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "email": "admin@localhost",
                            "password": "passwd",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let login_body: serde_json::Value =
            serde_json::from_slice(&to_bytes(login.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let old_refresh = login_body["data"]["refresh_token"].as_str().unwrap();

        let refresh_request = |token: &str| {
            Request::post("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "refresh_token": token }).to_string(),
                ))
                .unwrap()
        };
        let refreshed = app
            .clone()
            .oneshot(refresh_request(&old_refresh))
            .await
            .unwrap();
        assert_eq!(refreshed.status(), StatusCode::OK);
        assert_eq!(refreshed.headers()[header::CACHE_CONTROL], "no-store");
        assert!(refreshed.headers().get(header::SET_COOKIE).is_none());
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(refreshed.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let new_refresh = body["data"]["refresh_token"].as_str().unwrap();
        let new_access = body["data"]["access_token"].as_str().unwrap();
        assert_ne!(new_refresh, old_refresh);
        assert_eq!(body["data"]["token_type"], "Bearer");
        assert_eq!(
            body["data"]["expires_in"],
            container.config.jwt.access_token_ttl_seconds
        );
        assert_eq!(
            container.jwt_codec.verify(new_access).unwrap().token_use,
            TokenUse::Access
        );

        let reused = app
            .clone()
            .oneshot(refresh_request(&old_refresh))
            .await
            .unwrap();
        assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);
        let next = app.oneshot(refresh_request(new_refresh)).await.unwrap();
        assert_eq!(next.status(), StatusCode::OK);
    }
}
