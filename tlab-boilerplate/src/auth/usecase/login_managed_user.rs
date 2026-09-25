use std::sync::Arc;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::auth::repository::{refresh_token_repository, user_repository};
use crate::{app_container::AppDB, auth::entity};

//dummy login password
const DUMMY_LOGIN_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$o90CMRSOfYxJu7s/gJVLzA$QRg9RtFOtSICuubzdE+iJbUo0ct1tJxphWFyp/OLCXo";

#[derive(Debug, Clone)]
pub struct LoginManagedUserCommand {
    pub email: String,
    pub password: String,
}

pub struct LoginManagedUserResult {
    pub account: entity::UserAccount,
    pub tokens: tlab::jwt::TokenPair,
}

pub struct LoginManagedUserUsecase {
    app_db: Arc<AppDB>,
    password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
    jwt_codec: Arc<tlab::jwt::JwtCodec>,
}

impl LoginManagedUserUsecase {
    pub fn new(
        app_db: Arc<AppDB>,
        password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
        jwt_codec: Arc<tlab::jwt::JwtCodec>,
    ) -> Self {
        Self {
            app_db,
            password_hasher,
            jwt_codec,
        }
    }

    pub async fn execute(
        &self,
        command: &LoginManagedUserCommand,
    ) -> tlab::Result<LoginManagedUserResult> {
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

        let account = managed_login_user.account;
        let tokens = self.jwt_codec.issue_pair(&account.id.to_string())?;
        let token_hash: [u8; 32] = Sha256::digest(tokens.refresh.token.as_bytes()).into();
        let expires_at = i64::try_from(tokens.refresh.expires_at)
            .ok()
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
            .ok_or_else(|| tlab::Error::IllegalState("invalid refresh token expiration".into()))?;

        refresh_token_repository::save(&mut conn.context(), account.id, &token_hash, expires_at)
            .await?;

        Ok(LoginManagedUserResult { account, tokens })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::usecase::{CreateManagedUserCommand, CreateManagedUserUsecase};
    use tlab::hash::{Argon2Config, Argon2PasswordHasher, PasswordHasher};
    use tlab::jwt::{EdDsaKeyFiles, JwtCodec, JwtConfig, TokenUse};

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
        sqlx::raw_sql(include_str!("../migrations/003_create_refresh_token.sql"))
            .execute(conn.context().backend())
            .await
            .unwrap();
        drop(conn);
        app_db
    }

    fn jwt_codec() -> Arc<JwtCodec> {
        let directory = std::env::temp_dir().join(format!(
            "tlab-login-jwt-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir(&directory).unwrap();
        let key_files = EdDsaKeyFiles {
            private_key: directory.join("private.pem").to_string_lossy().into_owned(),
            public_key: directory.join("public.pem").to_string_lossy().into_owned(),
            generate_if_missing: true,
        };
        let codec = JwtCodec::new(&JwtConfig {
            key_files: key_files.clone(),
            issuer: "tlab-boilerplate".into(),
            audience: "tlab-boilerplate".into(),
            access_token_ttl_seconds: 60,
            refresh_token_ttl_seconds: 3600,
        })
        .unwrap();
        std::fs::remove_file(key_files.private_key).unwrap();
        std::fs::remove_file(key_files.public_key).unwrap();
        std::fs::remove_dir(directory).unwrap();
        Arc::new(codec)
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
        refresh_token_repository::delete(&mut tx.context(), user_id)
            .await
            .unwrap();
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
        let jwt_codec = jwt_codec();
        let usecase = LoginManagedUserUsecase::new(app_db.clone(), hasher, jwt_codec.clone());

        let logged_in = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "correct password".into(),
            })
            .await
            .unwrap();

        assert_eq!(logged_in.account.id, account.id);
        assert_eq!(logged_in.account.email, account.email);
        let access = jwt_codec.verify(&logged_in.tokens.access.token).unwrap();
        let refresh = jwt_codec.verify(&logged_in.tokens.refresh.token).unwrap();
        assert_eq!(access.sub, account.id.to_string());
        assert_eq!(access.token_use, TokenUse::Access);
        assert_eq!(refresh.token_use, TokenUse::Refresh);

        let mut conn = app_db.conn().await.unwrap();
        let stored =
            refresh_token_repository::find_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap()
                .unwrap();
        let expected_hash: [u8; 32] =
            Sha256::digest(logged_in.tokens.refresh.token.as_bytes()).into();
        assert_eq!(stored.token_hash, expected_hash);
        assert_eq!(stored.expires_at.timestamp(), refresh.exp as i64);
        drop(conn);
        delete_user(&app_db, account.id).await;
    }

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn rejects_login_for_missing_account() {
        let app_db = database().await;
        let usecase = LoginManagedUserUsecase::new(app_db, password_hasher(), jwt_codec());

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
        let usecase = LoginManagedUserUsecase::new(app_db.clone(), hasher, jwt_codec());

        let result = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "wrong password".into(),
            })
            .await;

        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));
        let mut conn = app_db.conn().await.unwrap();
        assert!(
            refresh_token_repository::find_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap()
                .is_none()
        );
        drop(conn);
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
