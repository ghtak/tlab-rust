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

    let access = build_cookie(
        "access_token",
        access_token,
        container.config.jwt.access_token_ttl_seconds,
    );

    jar.add(access).add(refresh)
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
    if matches!(
        req.uri().path(),
        "/api/v1/auth/login" | "/api/v1/auth/logout" | "/api/v1/auth/refresh"
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
    let tokens = RefreshTokenUsecase::new(container.database.clone(), container.jwt_codec.clone())
        .execute(&RefreshTokenCommand {
            refresh_token: refresh_token.value().to_owned(),
        })
        .await;
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(tlab::Error::InvalidToken) => return next.run(req).await,
        Err(error) => {
            tracing::error!(?error, "Failed to refresh auth cookies");
            return ApiResponse::internal_error("failed to refresh auth cookies").into_response();
        }
    };

    let claims = container
        .jwt_codec
        .verify(&tokens.access.token)
        .ok()
        .and_then(|jwt| AccessClaims::from_jwt(jwt).ok());
    let Some(claims) = claims else {
        tracing::error!("Failed to validate refreshed access token");
        return ApiResponse::internal_error("failed to refresh auth cookies").into_response();
    };
    req.extensions_mut().insert(claims);
    // 갱신된 쿠키는 내부 요청에 전달하지 않는다. 이번 요청의 인증은 AccessClaims로 처리하고,
    // 새 쿠키는 응답의 Set-Cookie로 전달한다.
    let response = next.run(req).await;
    (
        add_auth_cookies(
            CookieJar::new(),
            &tokens.access.token,
            &tokens.refresh.token,
            &container,
        ),
        response,
    )
        .into_response()
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

    #[tokio::test]
    async fn refreshes_cookie_without_an_access_token() {
        let (auth_app, container) = test_app::setup().await;
        let login = auth_app
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
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(login.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let old_refresh = body["data"]["refresh_token"].as_str().unwrap();
        let access = body["data"]["access_token"].as_str().unwrap();

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
            .route("/api/v1/auth/login", get(|| async { StatusCode::OK }))
            .route("/api/v1/auth/logout", get(|| async { StatusCode::OK }))
            .route("/api/v1/auth/refresh", get(|| async { StatusCode::OK }))
            .route_layer(middleware::from_fn_with_state(
                container.clone(),
                refresh_auth_cookies,
            ))
            .with_state(container.clone());
        let refreshed = app
            .clone()
            .oneshot(
                Request::get("/test")
                    .header(header::COOKIE, format!("refresh_token={old_refresh}"))
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
        assert_eq!(cookies.len(), 3);
        assert!(
            cookies
                .iter()
                .any(|cookie| cookie.name() == "theme" && cookie.value() == "dark")
        );
        let new_refresh = cookies
            .iter()
            .find(|cookie| cookie.name() == "refresh_token")
            .unwrap();
        assert_ne!(new_refresh.value(), old_refresh);
        assert_eq!(new_refresh.path(), Some("/"));
        assert_eq!(
            new_refresh.max_age(),
            Some(Duration::seconds(
                container.config.jwt.refresh_token_ttl_seconds as i64
            ))
        );
        assert!(cookies.iter().any(|cookie| cookie.name() == "access_token"));

        let unchanged = app
            .clone()
            .oneshot(
                Request::get("/test")
                    .header(
                        header::COOKIE,
                        format!(
                            "access_token={access}; refresh_token={}",
                            new_refresh.value()
                        ),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            unchanged
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .count(),
            1
        );

        let bearer_request = app
            .clone()
            .oneshot(
                Request::get("/test")
                    .header(header::AUTHORIZATION, format!("Bearer {access}"))
                    .header(
                        header::COOKIE,
                        format!("refresh_token={}", new_refresh.value()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            bearer_request
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .count(),
            1
        );

        for path in [
            "/api/v1/auth/login",
            "/api/v1/auth/logout",
            "/api/v1/auth/refresh",
        ] {
            let bypassed = app
                .clone()
                .oneshot(
                    Request::get(path)
                        .header(
                            header::COOKIE,
                            format!("refresh_token={}", new_refresh.value()),
                        )
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(bypassed.status(), StatusCode::OK);
            assert!(bypassed.headers().get(header::SET_COOKIE).is_none());
        }

        let mut request = Request::get("/test")
            .header(header::AUTHORIZATION, "Bearer invalid")
            .body(Body::empty())
            .unwrap();
        let jwt = container.jwt_codec.verify(access).unwrap();
        let subject = jwt.sub.clone();
        request
            .extensions_mut()
            .insert(AccessClaims::from_jwt(jwt).unwrap());
        let (mut parts, _) = request.into_parts();
        let claims = AccessClaims::from_request_parts(&mut parts, &container)
            .await
            .unwrap();
        assert_eq!(claims.jwt.sub, subject);
    }
}
