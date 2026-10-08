use std::sync::Arc;

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;

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

async fn get_auth_url(State(app): State<Arc<AppContainer>>) -> Result<Redirect, ApiResponse<()>> {
    let state = uuid::Uuid::new_v4().to_string();
    let auth_url = tlab::google_oauth2::get_auth_url(&app.config.google_oauth2, &state);
    Ok(Redirect::temporary(auth_url.as_str()))
}

async fn handle_oauth2_callback(
    State(app): State<Arc<AppContainer>>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Some(error) = params.get("error") {
        return login_error_redirect(if error == "access_denied" {
            "google_cancelled"
        } else {
            "google_failed"
        });
    }
    let Some(code) = params.get("code") else {
        return login_error_redirect("google_failed");
    };
    let tokens = LoginGoogleUserUsecase::new(
        app.database.clone(),
        app.token_service.clone(),
        app.config.google_oauth2.clone(),
    )
    .execute(&LoginGoogleUserCommand {
        code: code.clone(),
        state: params.get("state").cloned(),
    })
    .await;
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(tlab::Error::Conflict(_)) => return login_error_redirect("account_exists"),
        Err(tlab::Error::InvalidToken | tlab::Error::InvalidCredentials) => {
            return login_error_redirect("google_failed");
        }
        Err(error) => {
            tracing::error!(?error, "Failed to log in Google user");
            return login_error_redirect("google_failed");
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

fn login_error_redirect(error: &str) -> Response {
    Redirect::temporary(&format!("http://localhost:3000/login?error={error}")).into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
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
}
