use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{entity::UserStatus, repository::user_repository},
};

pub struct SetUserStatusCommand {
    pub user_account_id: i64,
    pub actor_id: i64,
    pub status: UserStatus,
}

pub struct SetUserStatusUsecase {
    app_db: Arc<AppDB>,
}

impl SetUserStatusUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(&self, command: &SetUserStatusCommand) -> tlab::Result<()> {
        let mut tx = self.app_db.tx().await?;
        let mut user = user_repository::find_user_account_for_update(
            &mut tx.context(),
            command.user_account_id,
        )
        .await?
        .ok_or(tlab::Error::NotFound("user".into()))?;

        if command.status == UserStatus::Withdrawn
            || user.status == UserStatus::Withdrawn
            || user.status == command.status
            || (user.email == "admin@localhost" && command.status == UserStatus::Suspended)
        {
            return Err(tlab::Error::InvalidOperation(
                "user status cannot be changed".into(),
            ));
        }

        user.status = command.status;
        user.update_by = Some(command.actor_id);
        user_repository::save_user_account(&mut tx.context(), &user).await?;
        tx.commit().await?;
        Ok(())
    }
}
