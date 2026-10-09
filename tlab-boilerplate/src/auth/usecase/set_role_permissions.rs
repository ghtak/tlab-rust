use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{
        repository::{permission_repository, role_repository},
        service::RbacService,
    },
};

pub struct SetRolePermissionsCommand {
    pub role_id: i64,
    pub permission_ids: Vec<i64>,
}

pub struct SetRolePermissionsUsecase {
    app_db: Arc<AppDB>,
}

impl SetRolePermissionsUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(&self, command: &SetRolePermissionsCommand) -> tlab::Result<()> {
        let mut tx = self.app_db.tx().await?;
        let role = role_repository::find_by_id_for_update(&mut tx.context(), command.role_id)
            .await?
            .ok_or(tlab::Error::NotFound("role".into()))?;

        let permissions = permission_repository::find_all_by_ids_for_key_share(
            &mut tx.context(),
            &command.permission_ids,
        )
        .await?;
        if permissions.len() != command.permission_ids.len() {
            return Err(tlab::Error::NotFound("permission".into()));
        }
        RbacService.validate_role_permission_change(&role, &permissions)?;

        role_repository::replace_permissions(&mut tx.context(), role.id, &command.permission_ids)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
