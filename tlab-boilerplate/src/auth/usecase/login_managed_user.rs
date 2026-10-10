use std::sync::Arc;

use crate::app_container::AppDB;
use crate::auth::{
    entity::{self, Provider, UserStatus},
    repository::user_repository,
    service::TokenService,
};

//dummy login password
const DUMMY_LOGIN_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$o90CMRSOfYxJu7s/gJVLzA$QRg9RtFOtSICuubzdE+iJbUo0ct1tJxphWFyp/OLCXo";

#[derive(Debug, Clone)]
pub struct LoginManagedUserCommand {
    pub email: String,
    pub password: String,
}

pub struct LoginManagedUserResult {
    pub tokens: tlab::jwt::TokenPair,
}

pub struct LoginManagedUserUsecase {
    app_db: Arc<AppDB>,
    password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
    token_service: Arc<TokenService>,
}

impl LoginManagedUserUsecase {
    pub fn new(
        app_db: Arc<AppDB>,
        password_hasher: Arc<dyn tlab::hash::PasswordHasher>,
        token_service: Arc<TokenService>,
    ) -> Self {
        Self {
            app_db,
            password_hasher,
            token_service,
        }
    }

    pub async fn execute(
        &self,
        command: &LoginManagedUserCommand,
    ) -> tlab::Result<LoginManagedUserResult> {
        let mut tx = self.app_db.tx().await?;
        let user = user_repository::find_user_by_email(&mut tx.context(), &command.email).await?;

        let Some(user) = user else {
            drop(tx);
            let _ = self
                .password_hasher
                .verify(&command.password, DUMMY_LOGIN_HASH);
            return Err(tlab::Error::InvalidCredentials);
        };
        let entity::User {
            account,
            identities,
            ..
        } = user;
        let managed = identities
            .into_iter()
            .find(|identity| identity.provider == Provider::Managed);
        let Some(mut identity) = managed else {
            let _ = self
                .password_hasher
                .verify(&command.password, DUMMY_LOGIN_HASH);
            return Err(tlab::Error::InvalidCredentials);
        };
        let credential = user_repository::find_user_credential_by_user_identity_id(
            &mut tx.context(),
            identity.id,
        )
        .await?;
        let Some(credential) =
            credential.filter(|credential| credential.provider == Provider::Managed)
        else {
            let _ = self
                .password_hasher
                .verify(&command.password, DUMMY_LOGIN_HASH);
            return Err(tlab::Error::InvalidCredentials);
        };

        if account.status != UserStatus::Active {
            return Err(tlab::Error::InvalidCredentials);
        }

        self.password_hasher
            .verify(&command.password, &credential.password_hash)?;

        let tokens = self
            .token_service
            .issue_login_tokens(&mut tx.context(), account.id)
            .await?;
        identity.last_login_at = Some(chrono::Utc::now());
        user_repository::save_user_identity(&mut tx.context(), &identity).await?;
        tx.commit().await?;
        Ok(LoginManagedUserResult { tokens })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::usecase::{
        CreateManagedUserCommand, CreateManagedUserUsecase, LogoutUserCommand, LogoutUserUsecase,
    };
    use crate::auth::{access_claims::AppClaims, entity, repository::refresh_token_repository};
    use crate::test_db;
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use tlab::hash::{Argon2Config, Argon2PasswordHasher, PasswordHasher};
    use tlab::jwt::{EdDsaKeyFiles, JwtCodec, JwtConfig, TokenUse};
    use uuid::Uuid;

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

    async fn delete_user(app_db: &AppDB, account: &entity::UserAccount) {
        let mut tx = app_db.tx().await.unwrap();
        let mut context = tx.context();
        let identity_id = user_repository::find_user_by_email(&mut context, &account.email)
            .await
            .unwrap()
            .unwrap()
            .identities
            .into_iter()
            .find(|identity| identity.provider == entity::Provider::Managed)
            .unwrap()
            .id;
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

    #[tokio::test]
    async fn logs_in_managed_user_with_correct_password() {
        let app_db = test_db::connect().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let mut conn = app_db.conn().await.unwrap();
        let role_id: i64 = sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
            .fetch_one(conn.context().backend())
            .await
            .unwrap();
        sqlx::query("INSERT INTO tlab_user_role (user_account_id, role_id) VALUES ($1, $2)")
            .bind(account.id)
            .bind(role_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
        drop(conn);
        let jwt_codec = jwt_codec();
        let usecase = LoginManagedUserUsecase::new(
            app_db.clone(),
            hasher,
            Arc::new(TokenService::new(jwt_codec.clone())),
        );

        let logged_in = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "correct password".into(),
            })
            .await
            .unwrap();

        let access = jwt_codec.verify(&logged_in.tokens.access.token).unwrap();
        let refresh = jwt_codec.verify(&logged_in.tokens.refresh.token).unwrap();
        assert_eq!(access.sub, account.id.to_string());
        assert_eq!(access.token_use, TokenUse::Access);
        let app = access.app.unwrap();
        assert_eq!(app["role_ids"], serde_json::json!([role_id]));
        let session_id = Uuid::parse_str(app["session_id"].as_str().unwrap()).unwrap();
        assert_eq!(refresh.token_use, TokenUse::Refresh);
        assert_eq!(refresh.app, Some(app));

        let mut conn = app_db.conn().await.unwrap();
        let stored =
            refresh_token_repository::find_all_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap();
        assert_eq!(stored.len(), 1);
        let stored = &stored[0];
        assert_eq!(stored.session_id, session_id);
        let expected_hash: [u8; 32] =
            Sha256::digest(logged_in.tokens.refresh.token.as_bytes()).into();
        assert_eq!(stored.token_hash, expected_hash);
        assert_eq!(stored.expires_at.timestamp(), refresh.exp as i64);
        let identity = user_repository::find_user_identity_by_provider_and_subject(
            &mut conn.context(),
            entity::Provider::Managed,
            &account.email,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(identity.last_login_at.is_some());
        drop(conn);
        delete_user(&app_db, &account).await;
    }

    #[tokio::test]
    async fn logout_removes_only_the_selected_session() {
        let app_db = test_db::connect().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let jwt_codec = jwt_codec();
        let login = LoginManagedUserUsecase::new(
            app_db.clone(),
            hasher,
            Arc::new(TokenService::new(jwt_codec.clone())),
        );
        let command = LoginManagedUserCommand {
            email: account.email.clone(),
            password: "correct password".into(),
        };
        let first = login.execute(&command).await.unwrap();
        let second = login.execute(&command).await.unwrap();
        assert_ne!(first.tokens.refresh.token, second.tokens.refresh.token);
        let session_id = |token: &str| {
            let app = jwt_codec.verify(token).unwrap().app.unwrap();
            serde_json::from_value::<AppClaims>(app).unwrap().session_id
        };
        let first_session_id = session_id(&first.tokens.access.token);
        let second_session_id = session_id(&second.tokens.access.token);
        assert_ne!(first_session_id, second_session_id);

        let mut conn = app_db.conn().await.unwrap();
        assert_eq!(
            refresh_token_repository::find_all_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap()
                .len(),
            2
        );
        drop(conn);

        let logout = LogoutUserUsecase::new(app_db.clone());
        let command = LogoutUserCommand {
            user_account_id: account.id,
            session_id: first_session_id,
        };
        logout.execute(&command).await.unwrap();
        logout.execute(&command).await.unwrap();

        let mut conn = app_db.conn().await.unwrap();
        let remaining =
            refresh_token_repository::find_all_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, second_session_id);
        drop(conn);
        let command = LogoutUserCommand {
            user_account_id: account.id,
            session_id: second_session_id,
        };
        logout.execute(&command).await.unwrap();
        delete_user(&app_db, &account).await;
    }

    #[tokio::test]
    async fn rejects_login_for_missing_account() {
        let usecase = LoginManagedUserUsecase::new(
            test_db::connect().await,
            password_hasher(),
            Arc::new(TokenService::new(jwt_codec())),
        );

        let result = usecase
            .execute(&LoginManagedUserCommand {
                email: unique_email(),
                password: "correct password".into(),
            })
            .await;

        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));
    }

    #[tokio::test]
    async fn rejects_login_without_managed_credential() {
        let app_db = test_db::connect().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let mut conn = app_db.conn().await.unwrap();
        let identity_id = user_repository::find_user_by_email(&mut conn.context(), &account.email)
            .await
            .unwrap()
            .unwrap()
            .identities[0]
            .id;
        user_repository::delete_user_credential_by_user_identity_id(
            &mut conn.context(),
            identity_id,
        )
        .await
        .unwrap();
        drop(conn);

        let result = LoginManagedUserUsecase::new(
            app_db.clone(),
            hasher,
            Arc::new(TokenService::new(jwt_codec())),
        )
        .execute(&LoginManagedUserCommand {
            email: account.email.clone(),
            password: "correct password".into(),
        })
        .await;
        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));

        delete_user(&app_db, &account).await;
    }

    #[tokio::test]
    async fn rejects_login_with_wrong_password() {
        let app_db = test_db::connect().await;
        let hasher = password_hasher();
        let account = create_user(&app_db, &hasher).await;
        let usecase = LoginManagedUserUsecase::new(
            app_db.clone(),
            hasher,
            Arc::new(TokenService::new(jwt_codec())),
        );

        let result = usecase
            .execute(&LoginManagedUserCommand {
                email: account.email.clone(),
                password: "wrong password".into(),
            })
            .await;

        assert!(matches!(result, Err(tlab::Error::InvalidCredentials)));
        let mut conn = app_db.conn().await.unwrap();
        assert!(
            refresh_token_repository::find_all_by_user_account_id(&mut conn.context(), account.id)
                .await
                .unwrap()
                .is_empty()
        );
        drop(conn);
        delete_user(&app_db, &account).await;
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
