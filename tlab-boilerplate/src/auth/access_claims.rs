use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{HeaderValue, header, request::Parts},
};
use serde::{Deserialize, Serialize};
use tlab::jwt::{JwtClaims, TokenUse};

use crate::{api_response::ApiResponse, app_container::AppContainer, auth::service::TokenService};

#[derive(Clone)]
pub struct AccessClaims {
    pub jwt: JwtClaims,
    pub app: AppClaims,
}

impl AccessClaims {
    pub(crate) fn from_jwt(jwt: JwtClaims) -> Result<Self, ApiResponse<()>> {
        let invalid_token = || ApiResponse::unauthorized("invalid token");
        if jwt.token_use != TokenUse::Access {
            return Err(invalid_token());
        }
        let app = jwt.app.as_ref().ok_or_else(invalid_token)?;
        let app = serde_json::from_value::<AppClaims>(app.clone()).map_err(|_| invalid_token())?;
        Ok(Self { jwt, app })
    }

    pub fn user_account_id(&self) -> Result<i64, ApiResponse<()>> {
        self.jwt
            .sub
            .parse()
            .map_err(|_| ApiResponse::unauthorized("invalid token"))
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AppClaims {
    pub role_ids: Vec<i64>,
    pub session_id: uuid::Uuid,
}

impl FromRequestParts<Arc<AppContainer>> for AccessClaims {
    type Rejection = ApiResponse<()>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppContainer>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(claims) = parts.extensions.get::<AccessClaims>() {
            return Ok(claims.clone());
        }
        extract_required_claims(parts.headers.get(header::AUTHORIZATION), &state.token_service)
            .await
    }
}

impl OptionalFromRequestParts<Arc<AppContainer>> for AccessClaims {
    type Rejection = ApiResponse<()>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppContainer>,
    ) -> Result<Option<Self>, Self::Rejection> {
        if let Some(claims) = parts.extensions.get::<AccessClaims>() {
            return Ok(Some(claims.clone()));
        }
        extract_claims(parts.headers.get(header::AUTHORIZATION), &state.token_service).await
    }
}

async fn extract_required_claims(
    authorization: Option<&HeaderValue>,
    token_service: &TokenService,
) -> Result<AccessClaims, ApiResponse<()>> {
    extract_claims(authorization, token_service)
        .await?
        .ok_or_else(|| ApiResponse::unauthorized("invalid token"))
}

async fn extract_claims(
    authorization: Option<&HeaderValue>,
    token_service: &TokenService,
) -> Result<Option<AccessClaims>, ApiResponse<()>> {
    let Some(authorization) = authorization else {
        return Ok(None);
    };

    let invalid_token = || ApiResponse::unauthorized("invalid token");
    let value = authorization.to_str().map_err(|_| invalid_token())?;
    let (scheme, token) = value.split_once(' ').ok_or_else(invalid_token)?;
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(invalid_token());
    }

    let jwt = token_service
        .verify_access(token)
        .await
        .map_err(|error| match error {
            tlab::Error::InvalidToken => invalid_token(),
            error => {
                tracing::error!(?error, "Failed to verify access token");
                ApiResponse::internal_error("failed to verify access token")
            }
        })?;
    AccessClaims::from_jwt(jwt).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tlab::jwt::{EdDsaKeyFiles, JwtCodec, JwtConfig};

    fn test_codec() -> JwtCodec {
        let directory =
            std::env::temp_dir().join(format!("tlab-optional-jwt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let config = JwtConfig {
            key_files: EdDsaKeyFiles {
                private_key: directory.join("private.pem").to_string_lossy().into_owned(),
                public_key: directory.join("public.pem").to_string_lossy().into_owned(),
                generate_if_missing: true,
            },
            issuer: "tlab".into(),
            audience: "tlab-api".into(),
            access_token_ttl_seconds: 60,
            refresh_token_ttl_seconds: 3600,
        };
        let codec = JwtCodec::new(&config).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        codec
    }

    #[tokio::test]
    async fn extracts_access_claims() {
        let codec = Arc::new(test_codec());
        let token_service = TokenService::new(codec.clone());
        let app = serde_json::json!({
            "role_ids": [1, 2],
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });
        let pair = codec.issue_pair("42", Some(app)).unwrap();
        token_service.activate_access(&pair.access).await.unwrap();
        let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
        let claims = extract_required_claims(Some(&access), &token_service)
            .await
            .unwrap();
        assert_eq!(claims.user_account_id().unwrap(), 42);
        assert_eq!(claims.jwt.token_use, TokenUse::Access);
        assert_eq!(claims.app.role_ids, vec![1, 2]);
        assert_eq!(
            claims.app.session_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[tokio::test]
    async fn rejects_non_numeric_subject() {
        let codec = Arc::new(test_codec());
        let token_service = TokenService::new(codec.clone());
        let app = serde_json::json!({
            "role_ids": [1],
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });
        let invalid_subject = codec.issue_pair("user-42", Some(app)).unwrap();
        let access =
            HeaderValue::from_str(&format!("Bearer {}", invalid_subject.access.token)).unwrap();
        assert_eq!(
            extract_claims(Some(&access), &token_service)
                .await
                .err()
                .unwrap()
                .status_code,
            axum::http::StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn rejects_missing_or_invalid_authorization() {
        let codec = Arc::new(test_codec());
        let token_service = TokenService::new(codec.clone());
        let pair = codec
            .issue_pair(
                "42",
                Some(serde_json::json!({
                    "role_ids": [1],
                    "session_id": "550e8400-e29b-41d4-a716-446655440000"
                })),
            )
            .unwrap();

        assert!(extract_claims(None, &token_service).await.unwrap().is_none());
        assert_eq!(
            extract_required_claims(None, &token_service)
                .await
                .err()
                .unwrap()
                .status_code,
            axum::http::StatusCode::UNAUTHORIZED
        );

        for value in [
            "Basic abc".to_owned(),
            "Bearer ".to_owned(),
            "Bearer invalid".to_owned(),
            format!("Bearer {}", pair.refresh.token),
        ] {
            let header = HeaderValue::from_str(&value).unwrap();
            let response = extract_claims(Some(&header), &token_service)
                .await
                .err()
                .unwrap();
            assert_eq!(response.status_code, axum::http::StatusCode::UNAUTHORIZED);
            assert_eq!(response.error.as_deref(), Some("invalid token"));
        }
    }

    #[tokio::test]
    async fn rejects_invalid_app_claims() {
        let codec = Arc::new(test_codec());
        let token_service = TokenService::new(codec.clone());
        let missing_app = codec.issue_pair("42", None).unwrap();
        token_service.activate_access(&missing_app.access).await.unwrap();
        let access =
            HeaderValue::from_str(&format!("Bearer {}", missing_app.access.token)).unwrap();
        assert_eq!(
            extract_claims(Some(&access), &token_service)
                .await
                .err()
                .unwrap()
                .status_code,
            axum::http::StatusCode::UNAUTHORIZED
        );
        for app in [
            serde_json::Value::Null,
            serde_json::json!({}),
            serde_json::json!({ "role_ids": [1] }),
            serde_json::json!({ "role_ids": "1" }),
            serde_json::json!({ "role_ids": [1, "2"] }),
            serde_json::json!({ "role_ids": [1], "session_id": "invalid" }),
        ] {
            let pair = codec.issue_pair("42", Some(app)).unwrap();
            token_service.activate_access(&pair.access).await.unwrap();
            let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
            let response = extract_claims(Some(&access), &token_service)
                .await
                .err()
                .unwrap();
            assert_eq!(response.status_code, axum::http::StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn accepts_only_the_latest_registered_access_token() {
        let codec = Arc::new(test_codec());
        let token_service = TokenService::new(codec.clone());
        let app = serde_json::json!({
            "role_ids": [1],
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });
        let first = codec.issue_access("42", Some(app.clone())).unwrap();
        let second = codec.issue_access("42", Some(app)).unwrap();
        let first_header = HeaderValue::from_str(&format!("Bearer {}", first.token)).unwrap();
        let second_header = HeaderValue::from_str(&format!("Bearer {}", second.token)).unwrap();

        assert!(extract_claims(Some(&first_header), &token_service).await.is_err());
        token_service.activate_access(&first).await.unwrap();
        assert!(extract_claims(Some(&first_header), &token_service).await.is_ok());
        token_service.activate_access(&second).await.unwrap();
        assert!(extract_claims(Some(&first_header), &token_service).await.is_err());
        assert!(extract_claims(Some(&second_header), &token_service).await.is_ok());
    }
}
