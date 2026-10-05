use std::sync::Arc;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tlab::jwt::{IssuedToken, JwtCodec, TokenPair, TokenUse};
use uuid::Uuid;

use crate::{
    app_container::AppDB,
    auth::{
        access_claims::AppClaims,
        repository::{rbac_repository, refresh_token_repository},
    },
};

pub struct RefreshTokenCommand {
    pub refresh_token: String,
}

pub struct RefreshTokenUsecase {
    app_db: Arc<AppDB>,
    jwt_codec: Arc<JwtCodec>,
}

impl RefreshTokenUsecase {
    pub fn new(app_db: Arc<AppDB>, jwt_codec: Arc<JwtCodec>) -> Self {
        Self { app_db, jwt_codec }
    }

    pub async fn execute(&self, command: &RefreshTokenCommand) -> tlab::Result<TokenPair> {
        let (user_account_id, session_id) = self.decode_refresh_token(&command.refresh_token)?;

        let mut conn = self.app_db.conn().await?;
        let role_ids =
            rbac_repository::find_role_ids_by_user_account_id(&mut conn.context(), user_account_id)
                .await?;
        let app = serde_json::to_value(AppClaims {
            role_ids,
            session_id,
        })
        .map_err(anyhow::Error::from)?;
        let tokens = self
            .jwt_codec
            .issue_pair(&user_account_id.to_string(), Some(app))?;
        let old_hash: [u8; 32] = Sha256::digest(command.refresh_token.as_bytes()).into();
        let new_hash: [u8; 32] = Sha256::digest(tokens.refresh.token.as_bytes()).into();
        let expires_at = i64::try_from(tokens.refresh.expires_at)
            .ok()
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
            .ok_or_else(|| tlab::Error::IllegalState("invalid refresh token expiration".into()))?;

        if !refresh_token_repository::rotate(
            &mut conn.context(),
            session_id,
            user_account_id,
            &old_hash,
            &new_hash,
            expires_at,
        )
        .await?
        {
            return Err(tlab::Error::InvalidToken);
        }

        Ok(tokens)
    }

    pub async fn execute_access(&self, command: &RefreshTokenCommand) -> tlab::Result<IssuedToken> {
        let (user_account_id, session_id) = self.decode_refresh_token(&command.refresh_token)?;
        let token_hash: [u8; 32] = Sha256::digest(command.refresh_token.as_bytes()).into();
        let mut conn = self.app_db.conn().await?;
        if !refresh_token_repository::is_valid(
            &mut conn.context(),
            session_id,
            user_account_id,
            &token_hash,
        )
        .await?
        {
            return Err(tlab::Error::InvalidToken);
        }
        let role_ids =
            rbac_repository::find_role_ids_by_user_account_id(&mut conn.context(), user_account_id)
                .await?;
        let app = serde_json::to_value(AppClaims {
            role_ids,
            session_id,
        })
        .map_err(anyhow::Error::from)?;
        self.jwt_codec
            .issue_access(&user_account_id.to_string(), Some(app))
    }

    fn decode_refresh_token(&self, token: &str) -> tlab::Result<(i64, Uuid)> {
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
}
