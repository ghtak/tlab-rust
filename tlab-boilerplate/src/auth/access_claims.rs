use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{HeaderValue, header, request::Parts},
};
use serde::{Deserialize, Serialize};
use tlab::jwt::{JwtClaims, JwtCodec, TokenUse};

use crate::{api_response::ApiResponse, app_container::AppContainer};

pub struct AccessClaims {
    pub jwt: JwtClaims,
    pub app: AppClaims,
}

impl AccessClaims {
    pub fn user_account_id(&self) -> Result<i64, ApiResponse<()>> {
        self.jwt
            .sub
            .parse()
            .map_err(|_| ApiResponse::unauthorized("invalid token"))
    }
}

#[derive(Deserialize, Serialize)]
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
        extract_required_claims(parts.headers.get(header::AUTHORIZATION), &state.jwt_codec)
    }
}

impl OptionalFromRequestParts<Arc<AppContainer>> for AccessClaims {
    type Rejection = ApiResponse<()>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppContainer>,
    ) -> Result<Option<Self>, Self::Rejection> {
        extract_claims(parts.headers.get(header::AUTHORIZATION), &state.jwt_codec)
    }
}

fn extract_required_claims(
    authorization: Option<&HeaderValue>,
    codec: &JwtCodec,
) -> Result<AccessClaims, ApiResponse<()>> {
    extract_claims(authorization, codec)?.ok_or_else(|| ApiResponse::unauthorized("invalid token"))
}

fn extract_claims(
    authorization: Option<&HeaderValue>,
    codec: &JwtCodec,
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

    let jwt = codec.verify(token).map_err(|_| invalid_token())?;
    if jwt.token_use != TokenUse::Access {
        return Err(invalid_token());
    }

    let app = jwt.app.as_ref().ok_or_else(invalid_token)?;
    let app = serde_json::from_value::<AppClaims>(app.clone()).map_err(|_| invalid_token())?;

    Ok(Some(AccessClaims { jwt, app }))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use tlab::jwt::{EdDsaKeyFiles, JwtConfig};

    #[test]
    fn extracts_only_valid_access_token() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("tlab-optional-jwt-{nonce}"));
        fs::create_dir(&directory).unwrap();
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
        let pair = codec.issue_pair("42", None).unwrap();

        assert!(extract_claims(None, &codec).unwrap().is_none());
        assert_eq!(
            extract_required_claims(None, &codec)
                .err()
                .unwrap()
                .status_code,
            axum::http::StatusCode::UNAUTHORIZED
        );

        let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
        assert_eq!(
            extract_claims(Some(&access), &codec)
                .err()
                .unwrap()
                .status_code,
            axum::http::StatusCode::UNAUTHORIZED
        );

        let app = serde_json::json!({
            "role_ids": [1, 2],
            "session_id": "550e8400-e29b-41d4-a716-446655440000"
        });
        let pair = codec.issue_pair("42", Some(app.clone())).unwrap();
        let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
        let claims = extract_required_claims(Some(&access), &codec).unwrap();
        assert_eq!(claims.jwt.sub, "42");
        assert_eq!(claims.user_account_id().unwrap(), 42);
        assert_eq!(claims.jwt.token_use, TokenUse::Access);
        assert_eq!(claims.app.role_ids, vec![1, 2]);
        assert_eq!(
            claims.app.session_id,
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );

        let invalid_subject = codec.issue_pair("user-42", Some(app)).unwrap();
        let access =
            HeaderValue::from_str(&format!("Bearer {}", invalid_subject.access.token)).unwrap();
        let claims = extract_claims(Some(&access), &codec).unwrap().unwrap();
        assert_eq!(
            claims.user_account_id().err().unwrap().status_code,
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
            let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
            let response = extract_claims(Some(&access), &codec).err().unwrap();
            assert_eq!(response.status_code, axum::http::StatusCode::UNAUTHORIZED);
        }

        for value in [
            "Basic abc".to_owned(),
            "Bearer ".to_owned(),
            "Bearer invalid".to_owned(),
            format!("Bearer {}", pair.refresh.token),
        ] {
            let header = HeaderValue::from_str(&value).unwrap();
            let response = extract_claims(Some(&header), &codec).err().unwrap();
            assert_eq!(response.status_code, axum::http::StatusCode::UNAUTHORIZED);
            assert_eq!(response.error.as_deref(), Some("invalid token"));
        }

        fs::remove_dir_all(directory).unwrap();
    }
}
