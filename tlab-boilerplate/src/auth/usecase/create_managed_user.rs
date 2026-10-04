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

        let user_account = user_repository::create_user_account(
            &mut tx.context(),
            &command.name,
            &command.email,
            entity::UserStatus::Active,
        )
        .await?;

        let user_identity = user_repository::create_user_identity(
            &mut tx.context(),
            user_account.id,
            entity::Provider::Managed,
            user_account.email.as_str(),
            Some(user_account.email.as_str()),
        )
        .await?;

        user_repository::create_user_credential(
            &mut tx.context(),
            user_identity.id,
            entity::Provider::Managed,
            &password_hash,
        )
        .await?;

        tx.commit().await?;
        Ok(user_account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migration::AUTH_MIGRATIONS, test_db};
    use tlab::hash::{PasswordHasher, Pbkdf2Config, Pbkdf2PasswordHasher};

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn creates_a_managed_user_with_a_verified_password() {
        let fixture = test_db::isolated_db(&[AUTH_MIGRATIONS[0].1, AUTH_MIGRATIONS[1].1]).await;
        let app_db = fixture.database.clone();

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
        let password_hash: String = sqlx::query_scalar(
            "SELECT credential.password_hash \
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

        drop(usecase);
        drop(app_db);
        fixture.cleanup().await;
    }
}
