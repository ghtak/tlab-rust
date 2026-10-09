use std::sync::Arc;

use chrono::Utc;

use tlab::jwt::{IssuedToken, TokenPair};

use crate::{
    app_container::AppDB,
    auth::{
        entity::{RefreshToken, UserStatus},
        repository::refresh_token_repository::{self, RefreshTokenCondition},
        service::TokenService,
    },
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
        let old_hash = TokenService::token_hash(&command.refresh_token);
        let condition = RefreshTokenCondition {
            user_account_id,
            session_id,
            token_hash: &old_hash,
            status: Some(UserStatus::Active),
        };

        let mut tx = self.app_db.tx().await?;
        let mut session =
            refresh_token_repository::find_by_condition_for_update(&mut tx.context(), &condition)
                .await?
                .ok_or(tlab::Error::InvalidToken)?;
        validate_session(&session)?;

        let tokens = self
            .token_service
            .issue_pair(&mut tx.context(), user_account_id, session_id)
            .await?;
        session.token_hash = TokenService::token_hash(&tokens.refresh.token).to_vec();
        session.expires_at = TokenService::token_expires_at(&tokens.refresh)?;
        refresh_token_repository::save(&mut tx.context(), &session).await?;
        tx.commit().await?;

        Ok(tokens)
    }

    pub async fn execute_access(&self, command: &RefreshTokenCommand) -> tlab::Result<IssuedToken> {
        let (user_account_id, session_id) = self
            .token_service
            .decode_refresh_token(&command.refresh_token)?;
        let token_hash = TokenService::token_hash(&command.refresh_token);
        let condition = RefreshTokenCondition {
            user_account_id,
            session_id,
            token_hash: &token_hash,
            status: Some(UserStatus::Active),
        };
        let mut conn = self.app_db.conn().await?;
        let session = refresh_token_repository::find_by_condition(&mut conn.context(), &condition)
            .await?
            .ok_or(tlab::Error::InvalidToken)?;
        validate_session(&session)?;
        self.token_service
            .issue_access(&mut conn.context(), user_account_id, session_id)
            .await
    }
}

fn validate_session(session: &RefreshToken) -> tlab::Result<()> {
    if session.expires_at <= Utc::now() {
        return Err(tlab::Error::InvalidToken);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_app;

    #[tokio::test]
    async fn rotates_once_and_rejects_suspended_account() {
        let (_, container) = test_app::setup().await;
        let mut conn = container.database.conn().await.unwrap();
        let user_account_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Alice', $1) RETURNING id",
        )
        .bind(format!("refresh-{}@example.com", uuid::Uuid::new_v4()))
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        let initial = container
            .token_service
            .issue_login_tokens(&mut conn.context(), user_account_id)
            .await
            .unwrap();
        drop(conn);

        let usecase =
            RefreshTokenUsecase::new(container.database.clone(), container.token_service.clone());
        let initial_command = RefreshTokenCommand {
            refresh_token: initial.refresh.token,
        };
        let (first, second) = tokio::join!(
            usecase.execute(&initial_command),
            usecase.execute(&initial_command)
        );
        assert!(matches!(
            (&first, &second),
            (Ok(_), Err(tlab::Error::InvalidToken)) | (Err(tlab::Error::InvalidToken), Ok(_))
        ));
        let rotated = first.or(second).unwrap();
        let rotated_command = RefreshTokenCommand {
            refresh_token: rotated.refresh.token,
        };
        usecase.execute_access(&rotated_command).await.unwrap();

        let mut conn = container.database.conn().await.unwrap();
        sqlx::query("UPDATE tlab_user_account SET status = 'suspended' WHERE id = $1")
            .bind(user_account_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
        drop(conn);
        assert!(matches!(
            usecase.execute_access(&rotated_command).await,
            Err(tlab::Error::InvalidToken)
        ));

        let mut conn = container.database.conn().await.unwrap();
        sqlx::query("DELETE FROM tlab_user_account WHERE id = $1")
            .bind(user_account_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
    }
}
