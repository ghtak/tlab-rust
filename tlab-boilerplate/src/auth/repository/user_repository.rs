use tlab::sqlxdb;

use crate::{app_container::AppDBCtx, auth::entity};

pub async fn create_user_account(
    context: &mut AppDBCtx<'_>,
    name: &str,
    email: &str,
    status: entity::UserStatus,
) -> tlab::Result<entity::UserAccount> {
    let row = sqlx::query!(
        r#"INSERT INTO tlab_user_account (name, email, status)
           VALUES ($1, $2, $3)
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
        name,
        email,
        status.as_str()
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    let status = row.status.parse().map_err(|_| {
        tlab::Error::IllegalState(format!("invalid user status: {}", row.status).into())
    })?;
    Ok(entity::UserAccount {
        id: row.id,
        name: row.name,
        email: row.email,
        status,
        created_at: row.created_at,
        updated_at: row.updated_at,
        create_by: row.create_by,
        update_by: row.update_by,
    })
}

pub async fn update_user_account(
    context: &mut AppDBCtx<'_>,
    user_account: &entity::UserAccount,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query!(
        r#"UPDATE tlab_user_account
           SET name = $1, email = $2, status = $3, update_by = $4, updated_at = CURRENT_TIMESTAMP
           WHERE id = $5
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
        &user_account.name,
        &user_account.email,
        user_account.status.as_str(),
        user_account.update_by,
        user_account.id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(|row| {
        let status = row.status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", row.status).into())
        })?;
        Ok(entity::UserAccount {
            id: row.id,
            name: row.name,
            email: row.email,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            create_by: row.create_by,
            update_by: row.update_by,
        })
    })
    .transpose()
}

pub async fn withdraw_user_account(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query!(
        r#"UPDATE tlab_user_account
           SET status = 'withdrawn', updated_at = CURRENT_TIMESTAMP
           WHERE id = $1
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
        user_account_id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(|row| {
        let status = row.status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", row.status).into())
        })?;
        Ok(entity::UserAccount {
            id: row.id,
            name: row.name,
            email: row.email,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            create_by: row.create_by,
            update_by: row.update_by,
        })
    })
    .transpose()
}

pub async fn find_managed_login_user(
    context: &mut AppDBCtx<'_>,
    email: &str,
) -> tlab::Result<Option<entity::ManagedLoginUser>> {
    let row = sqlx::query!(
        r#"SELECT account.id AS account_id,
                account.name AS account_name,
                account.email AS account_email,
                account.status AS account_status,
                account.created_at AS account_created_at,
                account.updated_at AS account_updated_at,
                account.create_by AS account_create_by,
                account.update_by AS account_update_by,
                credential.user_identity_id AS credential_identity_id,
                credential.password_hash AS password_hash,
                credential.password_changed_at AS password_changed_at
         FROM tlab_user_account AS account
         JOIN tlab_user_identity AS identity
           ON identity.user_account_id = account.id AND identity.provider = 'managed'
         JOIN tlab_user_credential AS credential
           ON credential.user_identity_id = identity.id AND credential.provider = 'managed'
         WHERE account.email = $1"#,
        email
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(|row| {
        let status = row.account_status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", row.account_status).into())
        })?;
        Ok(entity::ManagedLoginUser {
            account: entity::UserAccount {
                id: row.account_id,
                name: row.account_name,
                email: row.account_email,
                status,
                created_at: row.account_created_at,
                updated_at: row.account_updated_at,
                create_by: row.account_create_by,
                update_by: row.account_update_by,
            },
            credential: entity::UserCredential {
                user_identity_id: row.credential_identity_id,
                provider: entity::Provider::Managed,
                password_hash: row.password_hash,
                password_changed_at: row.password_changed_at,
            },
        })
    })
    .transpose()
}

pub async fn create_user_identity(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    provider: entity::Provider,
    provider_subject: &str,
    provider_email: Option<&str>,
) -> tlab::Result<entity::UserIdentity> {
    let row = sqlx::query!(
        r#"INSERT INTO tlab_user_identity (user_account_id, provider, provider_subject, provider_email)
           VALUES ($1, $2, $3, $4)
           RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
        user_account_id,
        provider.as_str(),
        provider_subject,
        provider_email
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    let provider = row.provider.parse().map_err(|_| {
        tlab::Error::IllegalState(format!("invalid provider: {}", row.provider).into())
    })?;
    Ok(entity::UserIdentity {
        id: row.id,
        user_account_id: row.user_account_id,
        provider,
        provider_subject: row.provider_subject,
        provider_email: row.provider_email,
        created_at: row.created_at,
        last_login_at: row.last_login_at,
    })
}

pub async fn update_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity: &entity::UserIdentity,
) -> tlab::Result<Option<entity::UserIdentity>> {
    let row = sqlx::query!(
        r#"UPDATE tlab_user_identity
           SET provider_email = $1, last_login_at = $2
           WHERE id = $3
           RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
        user_identity.provider_email.as_deref(),
        user_identity.last_login_at,
        user_identity.id
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(|row| {
        let provider = row.provider.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid provider: {}", row.provider).into())
        })?;
        Ok(entity::UserIdentity {
            id: row.id,
            user_account_id: row.user_account_id,
            provider,
            provider_subject: row.provider_subject,
            provider_email: row.provider_email,
            created_at: row.created_at,
            last_login_at: row.last_login_at,
        })
    })
    .transpose()
}

pub async fn delete_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query!(
        r#"DELETE FROM tlab_user_identity WHERE id = $1"#,
        user_identity_id
    )
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    if result.rows_affected() == 0 {
        return Err(tlab::Error::NotFound("user identity".into()));
    }
    Ok(())
}

pub async fn create_user_credential(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
    provider: entity::Provider,
    password_hash: &str,
) -> tlab::Result<entity::UserCredential> {
    let row = sqlx::query!(
        r#"INSERT INTO tlab_user_credential (user_identity_id, provider, password_hash)
           VALUES ($1, $2, $3)
           RETURNING user_identity_id, provider, password_hash, password_changed_at"#,
        user_identity_id,
        provider.as_str(),
        password_hash
    )
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    let provider = row.provider.parse().map_err(|_| {
        tlab::Error::IllegalState(format!("invalid provider: {}", row.provider).into())
    })?;
    Ok(entity::UserCredential {
        user_identity_id: row.user_identity_id,
        provider,
        password_hash: row.password_hash,
        password_changed_at: row.password_changed_at,
    })
}

pub async fn update_user_credential(
    context: &mut AppDBCtx<'_>,
    user_credential: &entity::UserCredential,
) -> tlab::Result<Option<entity::UserCredential>> {
    let row = sqlx::query!(
        r#"UPDATE tlab_user_credential
           SET password_hash = $1, password_changed_at = CURRENT_TIMESTAMP
           WHERE user_identity_id = $2 AND provider = $3
           RETURNING user_identity_id, provider, password_hash, password_changed_at"#,
        &user_credential.password_hash,
        user_credential.user_identity_id,
        user_credential.provider.as_str()
    )
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(|row| {
        let provider = row.provider.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid provider: {}", row.provider).into())
        })?;
        Ok(entity::UserCredential {
            user_identity_id: row.user_identity_id,
            provider,
            password_hash: row.password_hash,
            password_changed_at: row.password_changed_at,
        })
    })
    .transpose()
}

pub async fn delete_user_credential(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query!(
        r#"DELETE FROM tlab_user_credential WHERE user_identity_id = $1"#,
        user_identity_id
    )
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    if result.rows_affected() == 0 {
        return Err(tlab::Error::NotFound("user credential".into()));
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

        let login_user = find_managed_login_user(&mut context, &email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(login_user.account.id, account.id);
        assert_eq!(login_user.credential.password_hash, "updated-hash");
        assert!(
            find_managed_login_user(&mut context, "missing@example.com")
                .await
                .unwrap()
                .is_none()
        );

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
