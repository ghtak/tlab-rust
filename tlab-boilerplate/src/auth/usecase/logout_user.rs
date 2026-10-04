use std::sync::Arc;

use crate::{app_container::AppDB, auth::repository::refresh_token_repository};

pub struct LogoutUserUsecase {
    app_db: Arc<AppDB>,
}

impl LogoutUserUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(&self, user_account_id: i64) -> tlab::Result<()> {
        let mut conn = self.app_db.conn().await?;
        refresh_token_repository::delete(&mut conn.context(), user_account_id).await?;
        Ok(())
    }
}
