use std::sync::Arc;

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use time::Duration;

use crate::{
    api_response::ApiResponse,
    app_container::AppContainer,
    auth::{
        access_claims::AccessClaims,
        usecase::{RefreshTokenCommand, RefreshTokenUsecase},
    },
};

fn build_cookie(name: &'static str, value: &str, max_age_seconds: u64) -> Cookie<'static> {
    Cookie::build((name, value.to_string()))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(Duration::seconds(max_age_seconds as i64))
        .build()
}

pub fn add_auth_cookies(
    jar: CookieJar,
    access_token: &str,
    refresh_token: &str,
    container: &AppContainer,
) -> CookieJar {
    let refresh = build_cookie(
        "refresh_token",
        refresh_token,
        container.config.jwt.refresh_token_ttl_seconds,
    );

    add_access_cookie(jar.add(refresh), access_token, container)
}

fn add_access_cookie(jar: CookieJar, access_token: &str, container: &AppContainer) -> CookieJar {
    jar.add(build_cookie(
        "access_token",
        access_token,
        container.config.jwt.access_token_ttl_seconds,
    ))
}

pub fn clear_auth_cookies(jar: CookieJar) -> CookieJar {
    jar.add(build_cookie("access_token", "", 0))
        .add(build_cookie("refresh_token", "", 0))
}

pub async fn refresh_auth_cookies(
    State(container): State<Arc<AppContainer>>,
    jar: CookieJar,
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let is_logout = req.uri().path() == "/api/v1/auth/logout";
    if matches!(
        req.uri().path(),
        "/api/v1/auth/login" | "/api/v1/auth/refresh" | "/api/v1/auth/google/callback"
    ) {
        return next.run(req).await;
    }

    if req.headers().contains_key(header::AUTHORIZATION) {
        return next.run(req).await;
    }

    if let Some(claims) = jar
        .get("access_token")
        .and_then(|cookie| container.jwt_codec.verify(cookie.value()).ok())
        .and_then(|jwt| AccessClaims::from_jwt(jwt).ok())
    {
        req.extensions_mut().insert(claims);
        return next.run(req).await;
    }

    let Some(refresh_token) = jar.get("refresh_token") else {
        return next.run(req).await;
    };
    let access =
        RefreshTokenUsecase::new(container.database.clone(), container.token_service.clone())
            .execute_access(&RefreshTokenCommand {
                refresh_token: refresh_token.value().to_owned(),
            })
            .await;
    let access = match access {
        Ok(access) => access,
        Err(tlab::Error::InvalidToken) => return next.run(req).await,
        Err(error) => {
            tracing::error!(?error, "Failed to refresh auth cookies");
            return ApiResponse::internal_error("failed to refresh auth cookies").into_response();
        }
    };

    let claims = container
        .jwt_codec
        .verify(&access.token)
        .ok()
        .and_then(|jwt| AccessClaims::from_jwt(jwt).ok());
    let Some(claims) = claims else {
        tracing::error!("Failed to validate refreshed access token");
        return ApiResponse::internal_error("failed to refresh auth cookies").into_response();
    };
    req.extensions_mut().insert(claims);
    // 갱신된 access 쿠키는 내부 요청에 전달하지 않는다. 이번 요청의 인증은 AccessClaims로 처리한다.
    let response = next.run(req).await;
    if is_logout {
        response
    } else {
        (
            add_access_cookie(CookieJar::new(), &access.token, &container),
            response,
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_app;
    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::FromRequestParts,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    async fn login_tokens(app: Router) -> (String, String) {
        let response = app
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
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        (
            body["data"]["access_token"].as_str().unwrap().to_owned(),
            body["data"]["refresh_token"].as_str().unwrap().to_owned(),
        )
    }

    #[tokio::test]
    async fn refreshes_access_cookie_without_rotating_refresh_token() {
        let (auth_app, container) = test_app::setup().await;
        let (_, refresh_token) = login_tokens(auth_app).await;

        let app = Router::<Arc<AppContainer>>::new()
            .route(
                "/test",
                get(|_claims: AccessClaims| async {
                    (
                        CookieJar::new().add(Cookie::new("theme", "dark")),
                        StatusCode::OK,
                    )
                }),
            )
            .route_layer(middleware::from_fn_with_state(
                container.clone(),
                refresh_auth_cookies,
            ))
            .with_state(container.clone());
        let refreshed = app
            .oneshot(
                Request::get("/test")
                    .header(header::COOKIE, format!("refresh_token={refresh_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refreshed.status(), StatusCode::OK);
        let cookies: Vec<_> = refreshed
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| Cookie::parse(value.to_str().unwrap().to_owned()).unwrap())
            .collect();
        assert_eq!(cookies.len(), 2);
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.name() == "theme" && cookie.value() == "dark")
        );
        assert!(
            cookies
                .iter()
                .all(|cookie| cookie.name() != "refresh_token")
        );
        assert!(cookies.iter().any(|cookie| cookie.name() == "access_token"));
    }

    #[tokio::test]
    async fn access_claims_extension_takes_precedence_over_bearer() {
        let (app, container) = test_app::setup().await;
        let (access_token, _) = login_tokens(app).await;
        let claims =
            AccessClaims::from_jwt(container.jwt_codec.verify(&access_token).unwrap()).unwrap();
        let subject = claims.jwt.sub.clone();
        let mut request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer invalid")
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(claims);
        let (mut parts, _) = request.into_parts();
        let claims = AccessClaims::from_request_parts(&mut parts, &container)
            .await
            .unwrap();
        assert_eq!(claims.jwt.sub, subject);
    }
}
