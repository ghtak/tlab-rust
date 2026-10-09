use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tlab::jwt::{IssuedToken, TokenPair, TokenUse};
use uuid::Uuid;

use crate::{
    app_container::AppDBCtx,
    auth::{
        access_claims::AppClaims,
        entity::RefreshToken,
        repository::{refresh_token_repository, user_repository},
    },
};

pub struct TokenService {
    jwt_codec: Arc<tlab::jwt::JwtCodec>,
}

impl TokenService {
    pub fn new(jwt_codec: Arc<tlab::jwt::JwtCodec>) -> Self {
        Self { jwt_codec }
    }

    pub async fn issue_login_tokens(
        &self,
        context: &mut AppDBCtx<'_>,
        user_account_id: i64,
    ) -> tlab::Result<TokenPair> {
        let session_id = Uuid::new_v4();
        let tokens = self
            .issue_pair(context, user_account_id, session_id)
            .await?;
        let token_hash = Self::token_hash(&tokens.refresh.token);
        let expires_at = Self::token_expires_at(&tokens.refresh)?;

        let session =
            RefreshToken::new(session_id, user_account_id, token_hash.to_vec(), expires_at);
        refresh_token_repository::save(context, &session).await?;

        Ok(tokens)
    }

    pub fn decode_refresh_token(&self, token: &str) -> tlab::Result<(i64, Uuid)> {
        let claims = self.jwt_codec.verify(token)?;
        if claims.token_use != TokenUse::Refresh {
            return Err(tlab::Error::InvalidToken);
        }
        let user_account_id = claims
            .sub
            .parse::<i64>()
            .map_err(|_| tlab::Error::InvalidToken)?;
        let app = claims.app.ok_or(tlab::Error::InvalidToken)?;
        let session_id = serde_json::from_value::<AppClaims>(app)
            .map_err(|_| tlab::Error::InvalidToken)?
            .session_id;
        Ok((user_account_id, session_id))
    }

    pub async fn issue_pair(
        &self,
        context: &mut AppDBCtx<'_>,
        user_account_id: i64,
        session_id: Uuid,
    ) -> tlab::Result<TokenPair> {
        let app = self
            .app_claims(context, user_account_id, session_id)
            .await?;
        self.jwt_codec
            .issue_pair(&user_account_id.to_string(), Some(app))
    }

    pub async fn issue_access(
        &self,
        context: &mut AppDBCtx<'_>,
        user_account_id: i64,
        session_id: Uuid,
    ) -> tlab::Result<IssuedToken> {
        let app = self
            .app_claims(context, user_account_id, session_id)
            .await?;
        self.jwt_codec
            .issue_access(&user_account_id.to_string(), Some(app))
    }

    pub fn token_hash(token: &str) -> [u8; 32] {
        Sha256::digest(token.as_bytes()).into()
    }

    pub fn token_expires_at(token: &IssuedToken) -> tlab::Result<DateTime<Utc>> {
        i64::try_from(token.expires_at)
            .ok()
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
            .ok_or_else(|| tlab::Error::IllegalState("invalid refresh token expiration".into()))
    }

    async fn app_claims(
        &self,
        context: &mut AppDBCtx<'_>,
        user_account_id: i64,
        session_id: Uuid,
    ) -> tlab::Result<Value> {
        let role_ids = user_repository::find_all_role_ids(context, user_account_id).await?;
        serde_json::to_value(AppClaims {
            role_ids,
            session_id,
        })
        .map_err(anyhow::Error::from)
        .map_err(tlab::Error::from)
    }
}
