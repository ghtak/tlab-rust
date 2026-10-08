use std::sync::Arc;

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use time::Duration;

use crate::{
    api_response::ApiResponse,
    app_container::AppContainer,
    auth::usecase::{LoginGoogleUserCommand, LoginGoogleUserUsecase},
};

pub fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route(
            "/api/v1/auth/google/auth_url",
            axum::routing::get(get_auth_url),
        )
        .route(
            "/api/v1/auth/google/callback",
            axum::routing::get(handle_oauth2_callback),
        )
}

const STATE_COOKIE: &str = "google_oauth_state";
const CALLBACK_PATH: &str = "/api/v1/auth/google/callback";

async fn get_auth_url(
    State(app): State<Arc<AppContainer>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiResponse<()>> {
    let state = uuid::Uuid::new_v4().to_string();
    let auth_url = tlab::google_oauth2::get_auth_url(&app.config.google_oauth2, &state);
    let cookie = Cookie::build((STATE_COOKIE, state))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path(CALLBACK_PATH)
        .max_age(Duration::minutes(10))
        .build();
    Ok((jar.add(cookie), Redirect::temporary(auth_url.as_str())))
}

async fn handle_oauth2_callback(
    State(app): State<Arc<AppContainer>>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let expected_state = jar
        .get(STATE_COOKIE)
        .map(|cookie| cookie.value().to_owned());
    let jar = jar.remove(
        Cookie::build((STATE_COOKIE, ""))
            .path(CALLBACK_PATH)
            .build(),
    );
    if let Some(error) = params.get("error") {
        return login_error_redirect(
            jar,
            if error == "access_denied" {
                "google_cancelled"
            } else {
                "google_failed"
            },
        );
    }
    if expected_state.is_none()
        || expected_state.as_deref() != params.get("state").map(String::as_str)
    {
        return login_error_redirect(jar, "google_failed");
    }
    let Some(code) = params.get("code") else {
        return login_error_redirect(jar, "google_failed");
    };
    let tokens = LoginGoogleUserUsecase::new(
        app.database.clone(),
        app.token_service.clone(),
        app.config.google_oauth2.clone(),
    )
    .execute(&LoginGoogleUserCommand { code: code.clone() })
    .await;
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(tlab::Error::Conflict(_)) => return login_error_redirect(jar, "account_exists"),
        Err(tlab::Error::InvalidToken | tlab::Error::InvalidCredentials) => {
            return login_error_redirect(jar, "google_failed");
        }
        Err(error) => {
            tracing::error!(?error, "Failed to log in Google user");
            return login_error_redirect(jar, "google_failed");
        }
    };

    (
        crate::auth::cookie::add_auth_cookies(
            jar,
            &tokens.access.token,
            &tokens.refresh.token,
            &app,
        ),
        [(header::CACHE_CONTROL, "no-store")],
        Redirect::temporary("http://localhost:3000/"),
    )
        .into_response()
}

fn login_error_redirect(jar: CookieJar, error: &str) -> Response {
    (
        jar,
        Redirect::temporary(&format!("http://localhost:3000/login?error={error}")),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{CALLBACK_PATH, STATE_COOKIE};
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use axum_extra::extract::cookie::Cookie;
    use tower::ServiceExt;

    use crate::test_app;

    #[tokio::test]
    async fn callback_redirects_google_errors_to_login() {
        let (app, _) = test_app::setup().await;
        for (query, expected_error) in [
            ("error=access_denied", "google_cancelled"),
            ("error=server_error", "google_failed"),
            ("", "google_failed"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("/api/v1/auth/google/callback?{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
            assert_eq!(
                response.headers()[header::LOCATION],
                format!("http://localhost:3000/login?error={expected_error}")
            );
        }
    }

    #[tokio::test]
    async fn callback_rejects_state_from_another_login_attempt() {
        let (app, _) = test_app::setup().await;
        let start = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/google/auth_url")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start.status(), StatusCode::TEMPORARY_REDIRECT);
        let state_cookie =
            Cookie::parse(start.headers()[header::SET_COOKIE].to_str().unwrap()).unwrap();
        assert_eq!(state_cookie.name(), STATE_COOKIE);
        assert_eq!(state_cookie.path(), Some(CALLBACK_PATH));
        assert_eq!(state_cookie.http_only(), Some(true));
        assert_eq!(state_cookie.secure(), Some(true));
        assert!(
            start.headers()[header::LOCATION]
                .to_str()
                .unwrap()
                .contains(&format!("state={}", state_cookie.value()))
        );

        let callback = app
            .oneshot(
                Request::get("/api/v1/auth/google/callback?code=unused&state=wrong")
                    .header(
                        header::COOKIE,
                        format!("{STATE_COOKIE}={}", state_cookie.value()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(callback.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            callback.headers()[header::LOCATION],
            "http://localhost:3000/login?error=google_failed"
        );
        let cleared =
            Cookie::parse(callback.headers()[header::SET_COOKIE].to_str().unwrap()).unwrap();
        assert_eq!(cleared.name(), STATE_COOKIE);
        assert_eq!(cleared.path(), Some(CALLBACK_PATH));
        assert_eq!(cleared.value(), "");
    }
}
