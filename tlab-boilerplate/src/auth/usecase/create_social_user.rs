use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{entity, repository::user_repository},
};

#[derive(Debug, Clone)]
pub struct CreateSocialUserCommand {
    pub name: String,
    pub email: String,
    pub provider: entity::Provider,
    pub provider_subject: String,
}

pub struct CreateSocialUserUsecase {
    app_db: Arc<AppDB>,
}

impl CreateSocialUserUsecase {
    pub fn new(app_db: Arc<AppDB>) -> Self {
        Self { app_db }
    }

    pub async fn execute(
        &self,
        command: &CreateSocialUserCommand,
    ) -> tlab::Result<entity::UserAccount> {
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

        user_repository::save_user_identity(
            &mut tx.context(),
            &entity::UserIdentity::new(
                user_account.id,
                command.provider,
                command.provider_subject.clone(),
                Some(command.email.clone()),
            ),
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

    #[tokio::test]
    async fn creates_a_google_user_without_credential_or_role() {
        let app_db = test_db::connect().await;
        let usecase = CreateSocialUserUsecase::new(app_db.clone());
        let email = test_db::unique_email("social");
        let subject = uuid::Uuid::new_v4().to_string();

        let account = usecase
            .execute(&CreateSocialUserCommand {
                name: "Alice".into(),
                email: email.clone(),
                provider: entity::Provider::Google,
                provider_subject: subject.clone(),
            })
            .await
            .unwrap();

        assert_eq!(account.name, "Alice");
        assert_eq!(account.email, email);
        assert_eq!(account.status, entity::UserStatus::Active);

        let mut conn = app_db.conn().await.unwrap();
        let identity = user_repository::find_user_identity_by_provider_and_subject(
            &mut conn.context(),
            entity::Provider::Google,
            &subject,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(identity.user_account_id, account.id);
        assert_eq!(identity.provider_email.as_deref(), Some(email.as_str()));

        let credential_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tlab_user_credential WHERE user_identity_id = $1",
        )
        .bind(identity.id)
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        let role_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tlab_user_role WHERE user_account_id = $1")
                .bind(account.id)
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        assert_eq!(credential_count, 0);
        assert_eq!(role_count, 0);
        drop(conn);

        let other_subject = uuid::Uuid::new_v4().to_string();
        let duplicate_email = usecase
            .execute(&CreateSocialUserCommand {
                name: "Other user".into(),
                email: email.clone(),
                provider: entity::Provider::Google,
                provider_subject: other_subject.clone(),
            })
            .await;
        assert!(matches!(duplicate_email, Err(tlab::Error::Conflict(_))));
        let mut conn = app_db.conn().await.unwrap();
        assert!(
            user_repository::find_user_identity_by_provider_and_subject(
                &mut conn.context(),
                entity::Provider::Google,
                &other_subject,
            )
            .await
            .unwrap()
            .is_none()
        );
        drop(conn);

        let mut tx = app_db.tx().await.unwrap();
        user_repository::delete_user_identity_by_id(&mut tx.context(), identity.id)
            .await
            .unwrap();
        user_repository::delete_user_account_by_id(&mut tx.context(), account.id)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
}
