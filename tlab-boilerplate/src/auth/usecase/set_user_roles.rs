use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{
        repository::{role_repository, user_repository},
        service::RbacService,
    },
};

pub struct SetUserRolesCommand {
    pub user_account_id: i64,
    pub add_ids: Vec<i64>,
    pub remove_ids: Vec<i64>,
}

pub struct SetUserRolesUsecase {
    app_db: Arc<AppDB>,
}

impl SetUserRolesUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(&self, command: &SetUserRolesCommand) -> tlab::Result<()> {
        let mut tx = self.app_db.tx().await?;
        let user = user_repository::find_user_account_for_update(
            &mut tx.context(),
            command.user_account_id,
        )
        .await?
        .ok_or(tlab::Error::NotFound("user".into()))?;

        let ids: Vec<i64> = command
            .add_ids
            .iter()
            .chain(&command.remove_ids)
            .copied()
            .collect();
        let roles = role_repository::find_by_ids_for_update(&mut tx.context(), &ids).await?;
        if roles.len() != ids.len() {
            return Err(tlab::Error::NotFound("role".into()));
        }
        RbacService.validate_user_role_change(&user, &roles, &command.remove_ids)?;

        user_repository::remove_roles(&mut tx.context(), user.id, &command.remove_ids).await?;
        user_repository::add_roles(&mut tx.context(), user.id, &command.add_ids).await?;
        tx.commit().await?;
        Ok(())
    }
}
