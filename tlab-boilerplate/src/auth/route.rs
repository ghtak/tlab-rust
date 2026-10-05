use std::sync::Arc;

use crate::{
    api_response::{ApiResponse, ApiResult},
    app_container::AppContainer,
    auth::{
        self,
        access_claims::AccessClaims,
        permission,
        repository::user_repository,
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

#[derive(serde::Serialize)]
struct MeResponse {
    id: i64,
    name: String,
    email: String,
    status: String,
}

async fn me(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
) -> Result<impl IntoResponse, ApiResponse<()>> {
    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to load user")
    })?;
    let user = user_repository::find_user_account(&mut conn.context(), claims.user_account_id()?)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to load user");
            ApiResponse::internal_error("failed to load user")
        })?
        .ok_or_else(|| ApiResponse::unauthorized("invalid token"))?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        ApiResponse::data(MeResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            status: user.status.as_str().to_owned(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_app;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        response::Response,
    };
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use time::Duration;
    use tlab::jwt::TokenUse;
    use tower::ServiceExt;

    async fn login_as_admin(app: &Router) -> Response {
        let response = app
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
        assert_eq!(response.status(), StatusCode::OK);
        response
    }

    async fn response_json(response: Response) -> serde_json::Value {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    fn response_cookies(response: &Response) -> Vec<Cookie<'static>> {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| Cookie::parse(value.to_str().unwrap().to_owned()).unwrap())
            .collect()
    }

    fn assert_auth_cookies_cleared(response: &Response) {
        let cookies = response_cookies(response);
        assert_eq!(cookies.len(), 2);
        for name in ["access_token", "refresh_token"] {
            let cookie = cookies.iter().find(|cookie| cookie.name() == name).unwrap();
            assert_eq!(cookie.value(), "");
            assert_eq!(cookie.path(), Some("/"));
            assert_eq!(cookie.max_age(), Some(Duration::ZERO));
        }
    }

    fn refresh_request(token: &str) -> Request<Body> {
        Request::post("/api/v1/auth/refresh")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "refresh_token": token }).to_string(),
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn login_sets_auth_cookies() {
        let (app, container) = test_app::setup().await;
        let login = login_as_admin(&app).await;
        let login_cookies = response_cookies(&login);
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
    }

    #[tokio::test]
    async fn me_returns_user_without_management_permission() {
        let (app, container) = test_app::setup().await;
        let mut conn = container.database.conn().await.unwrap();
        let email = crate::test_db::unique_email("me");
        let user = user_repository::create_user_account(
            &mut conn.context(),
            "Regular User",
            &email,
            crate::auth::entity::UserStatus::Active,
        )
        .await
        .unwrap();
        drop(conn);
        let token = container
            .jwt_codec
            .issue_pair(
                &user.id.to_string(),
                Some(serde_json::json!({
                    "role_ids": [],
                    "session_id": uuid::Uuid::new_v4(),
                })),
            )
            .unwrap();

        let response = app
            .oneshot(
                Request::get("/api/v1/auth/me")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", token.access.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["data"]["id"], user.id);
        assert_eq!(body["data"]["name"], "Regular User");
        assert_eq!(body["data"]["email"], email);
        assert_eq!(body["data"]["status"], "active");
    }

    #[tokio::test]
    async fn logout_with_bearer_clears_auth_cookies() {
        let (app, _) = test_app::setup().await;
        let login = login_as_admin(&app).await;
        let login_cookies = response_cookies(&login);
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
        assert_auth_cookies_cleared(&logout);
    }

    #[tokio::test]
    async fn logout_with_refresh_cookie_clears_cookies_without_access_cookie() {
        let (app, _) = test_app::setup().await;
        let body = response_json(login_as_admin(&app).await).await;
        let refresh = body["data"]["refresh_token"].as_str().unwrap();

        let logout = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/logout")
                    .header(header::COOKIE, format!("refresh_token={refresh}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::OK);
        assert_auth_cookies_cleared(&logout);

        let reused = app.oneshot(refresh_request(refresh)).await.unwrap();
        assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn automatic_refresh_keeps_explicit_refresh_token_valid() {
        let (app, _) = test_app::setup().await;
        let login_body = response_json(login_as_admin(&app).await).await;
        let old_refresh = login_body["data"]["refresh_token"].as_str().unwrap();

        let automatic = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/me")
                    .header(header::COOKIE, format!("refresh_token={old_refresh}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(automatic.status(), StatusCode::OK);
        let cookies = response_cookies(&automatic);
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "access_token");

        let refreshed = app.oneshot(refresh_request(old_refresh)).await.unwrap();
        assert_eq!(refreshed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn refresh_rotates_api_token_and_rejects_reuse() {
        let (app, container) = test_app::setup().await;
        let login_body = response_json(login_as_admin(&app).await).await;
        let old_refresh = login_body["data"]["refresh_token"].as_str().unwrap();
        let refreshed = app
            .clone()
            .oneshot(refresh_request(&old_refresh))
            .await
            .unwrap();
        assert_eq!(refreshed.status(), StatusCode::OK);
        assert_eq!(refreshed.headers()[header::CACHE_CONTROL], "no-store");
        assert!(refreshed.headers().get(header::SET_COOKIE).is_none());
        let body = response_json(refreshed).await;
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
