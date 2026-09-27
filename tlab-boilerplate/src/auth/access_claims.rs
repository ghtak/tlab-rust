use std::sync::Arc;

use axum::{
    extract::OptionalFromRequestParts,
    http::{HeaderValue, header, request::Parts},
};
use tlab::jwt::{JwtClaims, JwtCodec, TokenUse};

use crate::{app_container::AppContainer, app_response::AppResponse};

pub struct AccessClaims(pub JwtClaims);

impl OptionalFromRequestParts<Arc<AppContainer>> for AccessClaims {
    type Rejection = AppResponse<()>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppContainer>,
    ) -> Result<Option<Self>, Self::Rejection> {
        extract_claims(parts.headers.get(header::AUTHORIZATION), &state.jwt_codec)
    }
}

fn extract_claims(
    authorization: Option<&HeaderValue>,
    codec: &JwtCodec,
) -> Result<Option<AccessClaims>, AppResponse<()>> {
    let Some(authorization) = authorization else {
        return Ok(None);
    };

    let invalid_token = || AppResponse::unauthorized("invalid token");
    let value = authorization.to_str().map_err(|_| invalid_token())?;
    let (scheme, token) = value.split_once(' ').ok_or_else(invalid_token)?;
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(invalid_token());
    }

    let claims = codec.verify(token).map_err(|_| invalid_token())?;
    if claims.token_use != TokenUse::Access {
        return Err(invalid_token());
    }

    Ok(Some(AccessClaims(claims)))
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
        let pair = codec.issue_pair("user-42").unwrap();

        assert!(extract_claims(None, &codec).unwrap().is_none());

        let access = HeaderValue::from_str(&format!("Bearer {}", pair.access.token)).unwrap();
        let claims = extract_claims(Some(&access), &codec).unwrap().unwrap().0;
        assert_eq!(claims.sub, "user-42");
        assert_eq!(claims.token_use, TokenUse::Access);

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
