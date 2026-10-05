use std::sync::Arc;

use axum::{extract::State, response::Response};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use time::Duration;

use crate::app_container::AppContainer;

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

    let access_token = jar.get("access_token").map(|c| c.value().to_string());
    let refresh_token = jar.get("refresh_token").map(|c| c.value().to_string());
    if access_token.is_none()
        || refresh_token.is_none()
        || container
            .jwt_codec
            .verify(access_token.unwrap().as_ref())
            .is_ok()
    {
        return next.run(req).await;
    }

    let refresh_token = refresh_token.unwrap();
    if let Ok(_claims) = container.jwt_codec.verify(refresh_token.as_ref()) {
        return next.run(req).await;
    }

    let res = next.run(req).await;
    res
}
