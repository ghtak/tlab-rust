use std::sync::Arc;
use uuid::Uuid;

use crate::{app_container::AppDB, auth::repository::refresh_token_repository};

#[derive(Debug, Clone)]
pub struct LogoutUserCommand {
    pub user_account_id: i64,
    pub session_id: Uuid,
}

pub struct LogoutUserUsecase {
    app_db: Arc<AppDB>,
}

impl LogoutUserUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(&self, command: &LogoutUserCommand) -> tlab::Result<()> {
        let mut conn = self.app_db.conn().await?;
        refresh_token_repository::delete_by_user_account_id_and_session_id(
            &mut conn.context(),
            command.user_account_id,
            command.session_id,
        )
        .await?;
        Ok(())
    }
}
