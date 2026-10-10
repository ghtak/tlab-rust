use crate::{
    app_container::AppDBCtx,
    auth::{entity, repository::user_repository},
};

pub struct SocialUserRegistration {
    pub name: String,
    pub email: String,
}

pub struct SocialUserService;

impl SocialUserService {
    pub async fn find_or_create(
        context: &mut AppDBCtx<'_>,
        provider: entity::Provider,
        provider_subject: &str,
        registration: Option<SocialUserRegistration>,
    ) -> tlab::Result<(entity::UserAccount, entity::UserIdentity)> {
        if let Some(identity) = user_repository::find_user_identity_by_provider_and_subject(
            context,
            provider,
            provider_subject,
        )
        .await?
        {
            let account =
                user_repository::find_user_account_by_id(context, identity.user_account_id)
                    .await?
                    .ok_or_else(|| {
                        tlab::Error::IllegalState("social user account not found".into())
                    })?;
            return Ok((account, identity));
        }

        let registration = registration.ok_or(tlab::Error::InvalidCredentials)?;
        let account = user_repository::save_user_account(
            context,
            &entity::UserAccount::new(
                registration.name,
                registration.email.clone(),
                entity::UserStatus::Active,
            ),
        )
        .await?;
        let identity = user_repository::save_user_identity(
            context,
            &entity::UserIdentity::new(
                account.id,
                provider,
                provider_subject.to_owned(),
                Some(registration.email),
            ),
        )
        .await?;
        Ok((account, identity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;

    #[tokio::test]
    async fn creates_and_reuses_social_user_in_caller_transaction() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let email = test_db::unique_email("social");
        let subject = uuid::Uuid::new_v4().to_string();

        let (account, identity) = SocialUserService::find_or_create(
            &mut tx.context(),
            entity::Provider::Google,
            &subject,
            Some(SocialUserRegistration {
                name: "Alice".into(),
                email: email.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(account.email, email);
        assert_eq!(identity.user_account_id, account.id);
        assert_eq!(identity.provider_email.as_deref(), Some(email.as_str()));

        let (same_account, same_identity) = SocialUserService::find_or_create(
            &mut tx.context(),
            entity::Provider::Google,
            &subject,
            None,
        )
        .await
        .unwrap();
        assert_eq!(same_account.id, account.id);
        assert_eq!(same_identity.id, identity.id);

        let credential_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tlab_user_credential WHERE user_identity_id = $1",
        )
        .bind(identity.id)
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        assert_eq!(credential_count, 0);
        tx.rollback().await.unwrap();

        let mut conn = database.conn().await.unwrap();
        assert!(
            user_repository::find_user_identity_by_provider_and_subject(
                &mut conn.context(),
                entity::Provider::Google,
                &subject,
            )
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn rejects_duplicate_email_without_creating_identity() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let email = test_db::unique_email("social-duplicate");
        let subject = uuid::Uuid::new_v4().to_string();
        SocialUserService::find_or_create(
            &mut tx.context(),
            entity::Provider::Google,
            &subject,
            Some(SocialUserRegistration {
                name: "Alice".into(),
                email: email.clone(),
            }),
        )
        .await
        .unwrap();

        let other_subject = uuid::Uuid::new_v4().to_string();
        let mut savepoint = tx.begin().await.unwrap();
        let result = SocialUserService::find_or_create(
            &mut savepoint.context(),
            entity::Provider::Google,
            &other_subject,
            Some(SocialUserRegistration {
                name: "Other user".into(),
                email,
            }),
        )
        .await;
        assert!(matches!(result, Err(tlab::Error::Conflict(_))));
        savepoint.rollback().await.unwrap();
        assert!(
            user_repository::find_user_identity_by_provider_and_subject(
                &mut tx.context(),
                entity::Provider::Google,
                &other_subject,
            )
            .await
            .unwrap()
            .is_none()
        );
        tx.rollback().await.unwrap();
    }
}
