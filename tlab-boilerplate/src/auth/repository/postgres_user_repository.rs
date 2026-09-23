use tlab::sqlxdb::{self};

use crate::{
    app_container::AppDBCtx,
    auth::{
        entity,
        repository::row::{UserAccountRow, UserCredentialRow, UserIdentityRow},
    },
};

pub async fn create_user_account(
    context: &mut AppDBCtx<'_>,
    name: &str,
    email: &str,
    status: entity::UserStatus,
) -> tlab::Result<entity::UserAccount> {
    sqlx::query_as!(
        UserAccountRow,
        "INSERT INTO tlab_user_account (name, email, status) \
         VALUES ($1, $2, $3) \
         RETURNING id, name, email, status, created_at, updated_at, create_by, update_by",
        name,
        email,
        status.as_str()
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .try_into()
}

pub async fn update_user_account(
    context: &mut AppDBCtx<'_>,
    user_account: &entity::UserAccount,
) -> tlab::Result<Option<entity::UserAccount>> {
    sqlx::query_as!(
        UserAccountRow,
        "UPDATE tlab_user_account \
         SET name = $1, email = $2, status = $3, update_by = $4, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $5 \
         RETURNING id, name, email, status, created_at, updated_at, create_by, update_by",
        &user_account.name,
        &user_account.email,
        user_account.status.as_str(),
        user_account.update_by,
        user_account.id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .map(|row| row.try_into())
    .transpose()
}

pub async fn withdraw_user_account(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::UserAccount>> {
    sqlx::query_as!(
        UserAccountRow,
        "UPDATE tlab_user_account \
         SET status = 'withdrawn', updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 \
         RETURNING id, name, email, status, created_at, updated_at, create_by, update_by",
        user_account_id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .map(|row| row.try_into())
    .transpose()
}

pub async fn create_user_identity(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    provider: entity::Provider,
    provider_subject: &str,
    provider_email: Option<&str>,
) -> tlab::Result<entity::UserIdentity> {
    sqlx::query_as!(
        UserIdentityRow,
        "INSERT INTO tlab_user_identity (user_account_id, provider, provider_subject, provider_email) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at",
        user_account_id,
        provider.as_str(),
        provider_subject,
        provider_email
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .try_into()
}

pub async fn update_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity: &entity::UserIdentity,
) -> tlab::Result<Option<entity::UserIdentity>> {
    sqlx::query_as!(
        UserIdentityRow,
        "UPDATE tlab_user_identity \
         SET provider_email = $1, last_login_at = $2 \
         WHERE id = $3 \
         RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at",
        user_identity.provider_email.as_deref(),
        user_identity.last_login_at,
        user_identity.id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .map(TryInto::try_into)
    .transpose()
}

pub async fn delete_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query!(
        "DELETE FROM tlab_user_identity WHERE id = $1",
        user_identity_id
    )
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    if result.rows_affected() == 0 {
        return Err(tlab::Error::not_found("user identity"));
    }
    Ok(())
}

pub async fn create_user_credential(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
    provider: entity::Provider,
    password_hash: &str,
) -> tlab::Result<entity::UserCredential> {
    sqlx::query_as!(
        UserCredentialRow,
        "INSERT INTO tlab_user_credential (user_identity_id, provider, password_hash) \
         VALUES ($1, $2, $3) \
         RETURNING user_identity_id, provider, password_hash, password_changed_at",
        user_identity_id,
        provider.as_str(),
        password_hash
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .try_into()
}

pub async fn update_user_credential(
    context: &mut AppDBCtx<'_>,
    user_credential: &entity::UserCredential,
) -> tlab::Result<Option<entity::UserCredential>> {
    sqlx::query_as!(
        UserCredentialRow,
        "UPDATE tlab_user_credential \
         SET password_hash = $1, password_changed_at = CURRENT_TIMESTAMP \
         WHERE user_identity_id = $2 AND provider = $3 \
         RETURNING user_identity_id, provider, password_hash, password_changed_at",
        &user_credential.password_hash,
        user_credential.user_identity_id,
        user_credential.provider.as_str()
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .map(TryInto::try_into)
    .transpose()
}

pub async fn delete_user_credential(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query!(
        "DELETE FROM tlab_user_credential WHERE user_identity_id = $1",
        user_identity_id
    )
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    if result.rows_affected() == 0 {
        return Err(tlab::Error::not_found("user credential"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_container::AppDB;

    async fn database() -> AppDB {
        AppDB::new(&tlab::sqlxdb::Config {
            url: "postgres://tlab:tlab@localhost:25432/tlab".into(),
            max_connections: 1,
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    #[ignore = "requires tests/docker-db-env PostgreSQL service"]
    async fn runs_user_repository_cud_with_postgres() {
        let database = database().await;
        let mut tx = database.tx().await.unwrap();
        let mut context = tx.context();

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

        let suffix = format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        );
        let email = format!("{suffix}@example.com");
        let mut account =
            create_user_account(&mut context, "Alice", &email, entity::UserStatus::Active)
                .await
                .unwrap();
        account.name = "Alice Updated".into();
        let account = update_user_account(&mut context, &account)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.name, "Alice Updated");

        let account = withdraw_user_account(&mut context, account.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.status, entity::UserStatus::Withdrawn);

        let mut identity = create_user_identity(
            &mut context,
            account.id,
            entity::Provider::Managed,
            &suffix,
            Some("alice@example.com"),
        )
        .await
        .unwrap();
        identity.provider_email = Some("updated@example.com".into());
        let identity = update_user_identity(&mut context, &identity)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            identity.provider_email.as_deref(),
            Some("updated@example.com")
        );

        let mut credential =
            create_user_credential(&mut context, identity.id, identity.provider, "initial-hash")
                .await
                .unwrap();
        credential.password_hash = "updated-hash".into();
        let credential = update_user_credential(&mut context, &credential)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(credential.password_hash, "updated-hash");

        delete_user_credential(&mut context, identity.id)
            .await
            .unwrap();
        delete_user_identity(&mut context, identity.id)
            .await
            .unwrap();
        assert!(
            delete_user_credential(&mut context, identity.id)
                .await
                .is_err()
        );

        drop(context);
        tx.rollback().await.unwrap();
    }
}
