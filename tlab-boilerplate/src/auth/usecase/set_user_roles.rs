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
    pub role_ids: Vec<i64>,
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
        let user = user_repository::find_user_account_by_id_for_update(
            &mut tx.context(),
            command.user_account_id,
        )
        .await?
        .ok_or(tlab::Error::NotFound("user".into()))?;

        let roles =
            role_repository::find_all_by_ids_for_key_share(&mut tx.context(), &command.role_ids)
                .await?;
        if roles.len() != command.role_ids.len() {
            return Err(tlab::Error::NotFound("role".into()));
        }
        RbacService.validate_user_role_change(&user, &roles)?;

        user_repository::replace_roles(&mut tx.context(), user.id, &command.role_ids).await?;
        tx.commit().await?;
        Ok(())
    }
}
