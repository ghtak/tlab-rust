use std::sync::Arc;
use uuid::Uuid;

use crate::{
    app_container::AppDB,
    auth::{repository::refresh_token_repository, service::TokenService},
};

#[derive(Debug, Clone)]
pub struct LogoutUserCommand {
    pub user_account_id: i64,
    pub session_id: Uuid,
    pub access_jti: String,
}

pub struct LogoutUserUsecase {
    app_db: Arc<AppDB>,
    token_service: Arc<TokenService>,
}

impl LogoutUserUsecase {
    pub fn new(app_db: Arc<AppDB>, token_service: Arc<TokenService>) -> Self {
        Self { app_db, token_service }
    }

    pub async fn execute(&self, command: &LogoutUserCommand) -> tlab::Result<()> {
        let mut conn = self.app_db.conn().await?;
        refresh_token_repository::delete_by_user_account_id_and_session_id(
            &mut conn.context(),
            command.user_account_id,
            command.session_id,
        )
        .await?;
        self.token_service
            .revoke_access(command.user_account_id, &command.access_jti)
            .await;
        Ok(())
    }
}
