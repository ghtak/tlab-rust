use std::sync::Arc;

use crate::auth::repository::user_repository;
use crate::{app_container::AppDB, auth::entity};

//dummy login password
const DUMMY_LOGIN_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$o90CMRSOfYxJu7s/gJVLzA$QRg9RtFOtSICuubzdE+iJbUo0ct1tJxphWFyp/OLCXo";

#[derive(Debug, Clone)]
pub struct LoginManagedUserCommand {
    pub email: String,
    pub password: String,
}

pub struct LoginManagedUserUsecase {
    app_db: Arc<AppDB>,
    password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
}

impl LoginManagedUserUsecase {
    pub fn new(app_db: Arc<AppDB>, password_hasher: Arc<dyn tlab::hash::PasswordHasher>) -> Self {
        Self {
            app_db,
            password_hasher,
        }
    }

    pub async fn execute(
        &self,
        command: &LoginManagedUserCommand,
    ) -> tlab::Result<entity::UserAccount> {
        let mut conn = self.app_db.conn().await?;
        let managed_login_user =
            user_repository::find_managed_login_user(&mut conn.context(), &command.email).await?;

        let Some(managed_login_user) = managed_login_user else {
            let _ = self
                .password_hasher
                .verify(&command.password, DUMMY_LOGIN_HASH);
            return Err(tlab::Error::InvalidCredentials);
        };

        self.password_hasher.verify(
            &command.password,
            &managed_login_user.credential.password_hash,
        )?;
        Ok(managed_login_user.account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::usecase::{CreateManagedUserCommand, CreateManagedUserUsecase};
    use tlab::hash::{Argon2Config, Argon2PasswordHasher, PasswordHasher};

    async fn database() -> Arc<AppDB> {
        let app_db = Arc::new(
            AppDB::new(&tlab::sqlxdb::Config {
                url: "postgres://tlab:tlab@localhost:25432/tlab".into(),
                max_connections: 1,
            })
            .await
            .unwrap(),
        );
        let mut conn = app_db.conn().await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/001_create_user_account.sql"))
            .execute(conn.context().backend())
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../migrations/002_create_user_identity_and_credential.sql"
        ))
        .execute(conn.context().backend())
        .await
        .unwrap();
        drop(conn);
        app_db
    }

    fn password_hasher() -> Arc<dyn PasswordHasher> {
        Arc::new(
            Argon2PasswordHasher::new(&Argon2Config {
                memory_cost_kib: 19_456,
                time_cost: 2,
                parallelism: 1,
                output_len: Some(32),
            })
            .unwrap(),
        )
    }

    fn unique_email() -> String {
        format!(
            "login-{}-{}@example.com",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        )
    }

    async fn create_user(
        app_db: &Arc<AppDB>,
        hasher: &Arc<dyn PasswordHasher>,
    ) -> entity::UserAccount {
        CreateManagedUserUsecase::new(app_db.clone(), hasher.clone())
            .execute(&CreateManagedUserCommand {
                name: "Alice".into(),
                email: unique_email(),
                password: "correct password".into(),
            })
            .await
            .unwrap()
    }

    async fn delete_user(app_db: &AppDB, user_id: i64) {
        let mut tx = app_db.tx().await.unwrap();
        sqlx::query(
            "DELETE FROM tlab_user_credential \
             WHERE user_identity_id IN (SELECT id FROM tlab_user_identity WHERE user_account_id = $1)",
        )
        .bind(user_id)
        .execute(tx.context().backend())
        .await
        .unwrap();
        sqlx::query("DELETE FROM tlab_user_identity WHERE user_account_id = $1")
            .bind(user_id)
            .execute(tx.context().backend())
            .await
            .unwrap();
        sqlx::query("DELETE FROM tlab_user_account WHERE id = $1")
            .bind(user_id)
            .execute(tx.context().backend())
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn logs_in_managed_user_with_correct_password() {
        let app_db = database().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let usecase = LoginManagedUserUsecase::new(app_db.clone(), hasher);

        let logged_in = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "correct password".into(),
            })
            .await
            .unwrap();

        assert_eq!(logged_in.id, account.id);
        assert_eq!(logged_in.email, account.email);
        delete_user(&app_db, account.id).await;
    }

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn rejects_login_for_missing_account() {
        let app_db = database().await;
        let usecase = LoginManagedUserUsecase::new(app_db, password_hasher());

        let result = usecase
            .execute(&LoginManagedUserCommand {
                email: unique_email(),
                password: "correct password".into(),
            })
            .await;

        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));
    }

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn rejects_login_with_wrong_password() {
        let app_db = database().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let usecase = LoginManagedUserUsecase::new(app_db.clone(), hasher);

        let result = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "wrong password".into(),
            })
            .await;

        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));
        delete_user(&app_db, account.id).await;
    }

    #[test]
    fn make_dummy_hash() {
        let password_hanher = tlab::hash::Argon2PasswordHasher::new(&Argon2Config {
            memory_cost_kib: 19_456,
            time_cost: 2,
            parallelism: 1,
            output_len: Some(32),
        })
        .unwrap();

        let hash = password_hanher.hash("dummy login password").unwrap();
        //$argon2id$v=19$m=19456,t=2,p=1$o90CMRSOfYxJu7s/gJVLzA$QRg9RtFOtSICuubzdE+iJbUo0ct1tJxphWFyp/OLCXo
        println!("{}", hash);
    }
}
