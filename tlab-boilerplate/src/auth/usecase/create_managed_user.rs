use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{entity, repository},
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
        command: CreateManagedUserCommand,
    ) -> tlab::Result<entity::UserAccount> {
        let password_hash = self.password_hasher.hash(&command.password)?;
        let mut tx = self.app_db.tx().await?;

        let user_account = repository::create_user_account(
            &mut tx.context(),
            &command.name,
            &command.email,
            entity::UserStatus::Active,
        )
        .await?;

        let user_identity = repository::create_user_identity(
            &mut tx.context(),
            user_account.id,
            entity::Provider::Managed,
            user_account.email.as_str(),
            Some(user_account.email.as_str()),
        )
        .await?;

        repository::create_user_credential(
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
    use tlab::hash::{PasswordHasher, Pbkdf2Config, Pbkdf2PasswordHasher};

    async fn database() -> Arc<AppDB> {
        Arc::new(
            AppDB::new(&tlab::sqlxdb::Config {
                url: "postgres://tlab:tlab@localhost:25432/tlab".into(),
                max_connections: 1,
            })
            .await
            .unwrap(),
        )
    }

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn creates_a_managed_user_with_a_verified_password() {
        let app_db = database().await;
        let mut connection = app_db.conn().await.unwrap();
        let mut context = connection.context();

        sqlx::raw_sql(include_str!("../migrations/001_create_user_account.sql"))
            .execute(context.backend())
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../migrations/002_create_user_identity_and_credential.sql"
        ))
        .execute(context.backend())
        .await
        .unwrap();
        drop(context);
        drop(connection);

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
            .execute(CreateManagedUserCommand {
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

        let mut cleanup = app_db.tx().await.unwrap();
        sqlx::query(
            "DELETE FROM tlab_user_credential \
             WHERE user_identity_id IN (SELECT id FROM tlab_user_identity WHERE user_account_id = $1)",
        )
        .bind(account.id)
        .execute(cleanup.context().backend())
        .await
        .unwrap();
        sqlx::query("DELETE FROM tlab_user_identity WHERE user_account_id = $1")
            .bind(account.id)
            .execute(cleanup.context().backend())
            .await
            .unwrap();
        sqlx::query("DELETE FROM tlab_user_account WHERE id = $1")
            .bind(account.id)
            .execute(cleanup.context().backend())
            .await
            .unwrap();
        cleanup.commit().await.unwrap();
    }
}
