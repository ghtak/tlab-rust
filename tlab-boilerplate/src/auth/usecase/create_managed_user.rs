use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{entity, repository::user_repository},
};

#[derive(Debug, Clone)]
pub struct CreateManagedUserCommand {
    pub name: String,
    pub email: String,
    pub password: String,
}

pub struct CreateManagedUserUsecase {
    app_db: Arc<AppDB>,
    password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
}

impl CreateManagedUserUsecase {
    pub fn new(app_db: Arc<AppDB>, password_hasher: Arc<dyn tlab::hash::PasswordHasher>) -> Self {
        Self {
            app_db,
            password_hasher,
        }
    }

    pub async fn execute(
        &self,
        command: &CreateManagedUserCommand,
    ) -> tlab::Result<entity::UserAccount> {
        let password_hash = self.password_hasher.hash(&command.password)?;
        let mut tx = self.app_db.tx().await?;

        let user_account = user_repository::save_user_account(
            &mut tx.context(),
            &entity::UserAccount::new(
                command.name.clone(),
                command.email.clone(),
                entity::UserStatus::Active,
            ),
        )
        .await?;

        let user_identity = user_repository::save_user_identity(
            &mut tx.context(),
            &entity::UserIdentity::new(
                user_account.id,
                entity::Provider::Managed,
                user_account.email.clone(),
                Some(user_account.email.clone()),
            ),
        )
        .await?;

        user_repository::save_user_credential(
            &mut tx.context(),
            &entity::UserCredential::new(user_identity.id, entity::Provider::Managed, password_hash),
        )
        .await?;

        tx.commit().await?;
        Ok(user_account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;
    use tlab::hash::{PasswordHasher, Pbkdf2Config, Pbkdf2PasswordHasher};

    #[tokio::test]
    async fn creates_a_managed_user_with_a_verified_password() {
        let app_db = test_db::connect().await;

        let password_hasher = Arc::new(Pbkdf2PasswordHasher::new(&Pbkdf2Config {
            iterations: 1_000,
            output_len: 32,
        }));
        let usecase = CreateManagedUserUsecase::new(app_db.clone(), password_hasher.clone());
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        );
        let email = format!("{suffix}@example.com");

        let account = usecase
            .execute(&CreateManagedUserCommand {
                name: "Alice".into(),
                email: email.clone(),
                password: "password".into(),
            })
            .await
            .unwrap();

        assert_eq!(account.name, "Alice");
        assert_eq!(account.email, email);
        assert_eq!(account.status, entity::UserStatus::Active);

        let mut connection = app_db.conn().await.unwrap();
        let (identity_id, password_hash): (i64, String) = sqlx::query_as(
            "SELECT credential.user_identity_id, credential.password_hash \
             FROM tlab_user_credential AS credential \
             JOIN tlab_user_identity AS identity ON identity.id = credential.user_identity_id \
             WHERE identity.user_account_id = $1 AND identity.provider = 'managed'",
        )
        .bind(account.id)
        .fetch_one(connection.context().backend())
        .await
        .unwrap();
        password_hasher.verify("password", &password_hash).unwrap();
        drop(connection);

        let mut tx = app_db.tx().await.unwrap();
        let mut context = tx.context();
        user_repository::delete_user_credential_by_user_identity_id(&mut context, identity_id)
            .await
            .unwrap();
        user_repository::delete_user_identity_by_id(&mut context, identity_id)
            .await
            .unwrap();
        user_repository::delete_user_account_by_id(&mut context, account.id)
            .await
            .unwrap();
        drop(context);
        tx.commit().await.unwrap();
    }
}
