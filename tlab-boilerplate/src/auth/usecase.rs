use std::sync::Arc;

use crate::{app_container::AppDB, auth::entity};

#[derive(Debug, Clone)]
pub struct CreateManagedUserCommand {
    pub name: String,
    pub email: String,
    pub password: String,
}

pub struct CreateManagedUserUsecase {
    app_db: Arc<AppDB>,
}

impl CreateManagedUserUsecase {
    async fn execute(&self, command: CreateManagedUserCommand) -> tlab::Result<entity::UserAccount> {
        let mut tx = self.app_db.tx().await?;
        // todo: insert user account, identity, and credential into the database
        tx.commit().await?;
        Ok(entity::UserAccount {
            id: 1,
            name: command.name,
            email: command.email,
            status: "active".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            create_by: None,
            update_by: None,
        })
    }
}

