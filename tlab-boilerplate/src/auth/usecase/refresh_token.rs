use std::sync::Arc;

use tlab::jwt::{IssuedToken, TokenPair};

use crate::{
    app_container::AppDB,
    auth::{repository::refresh_token_repository, service::TokenService},
};

pub struct RefreshTokenCommand {
    pub refresh_token: String,
}

pub struct RefreshTokenUsecase {
    app_db: Arc<AppDB>,
    token_service: Arc<TokenService>,
}

impl RefreshTokenUsecase {
    pub fn new(app_db: Arc<AppDB>, token_service: Arc<TokenService>) -> Self {
        Self {
            app_db,
            token_service,
        }
    }

    pub async fn execute(&self, command: &RefreshTokenCommand) -> tlab::Result<TokenPair> {
        let (user_account_id, session_id) = self
            .token_service
            .decode_refresh_token(&command.refresh_token)?;

        let mut conn = self.app_db.conn().await?;
        let tokens = self
            .token_service
            .issue_pair(&mut conn.context(), user_account_id, session_id)
            .await?;
        let old_hash = TokenService::token_hash(&command.refresh_token);
        let new_hash = TokenService::token_hash(&tokens.refresh.token);
        let expires_at = TokenService::token_expires_at(&tokens.refresh)?;

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
        let (user_account_id, session_id) = self
            .token_service
            .decode_refresh_token(&command.refresh_token)?;
        let token_hash = TokenService::token_hash(&command.refresh_token);
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
        self.token_service
            .issue_access(&mut conn.context(), user_account_id, session_id)
            .await
    }
}
