#![allow(dead_code)]

use chrono::{DateTime, Utc};
use tlab::{paging::Paging, sqlxdb};

use crate::{app_container::AppDBCtx, auth::entity};

#[derive(sqlx::FromRow)]
struct UserAccountRow {
    id: i64,
    name: String,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    create_by: Option<i64>,
    update_by: Option<i64>,
}

impl UserAccountRow {
    fn into_entity(self) -> tlab::Result<entity::UserAccount> {
        let status = self.status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", self.status).into())
        })?;
        Ok(entity::UserAccount {
            id: self.id,
            name: self.name,
            email: self.email,
            status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            create_by: self.create_by,
            update_by: self.update_by,
        })
    }
}

#[derive(sqlx::FromRow)]
struct UserListRow {
    #[sqlx(flatten)]
    account: UserAccountRow,
    roles: Vec<String>,
    providers: Vec<String>,
}

pub struct UserListItem {
    pub account: entity::UserAccount,
    pub roles: Vec<String>,
    pub providers: Vec<String>,
}

pub async fn find_user_accounts(
    context: &mut AppDBCtx<'_>,
    query: Option<&str>,
    status: Option<entity::UserStatus>,
    limit: i64,
    offset: i64,
) -> tlab::Result<Paging<UserListItem>> {
    let status = status.map(entity::UserStatus::as_str);
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM tlab_user_account
        WHERE ($1::TEXT IS NULL OR name ILIKE '%' || $1 || '%' OR email ILIKE '%' || $1 || '%')
          AND ($2::TEXT IS NULL OR status = $2)"#,
    )
    .bind(query)
    .bind(status)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    let rows: Vec<UserListRow> = sqlx::query_as(
        r#"SELECT account.id, account.name, account.email, account.status,
               account.created_at, account.updated_at, account.create_by, account.update_by,
               ARRAY(SELECT role.code::TEXT
                     FROM tlab_user_role AS link
                     JOIN tlab_role AS role ON role.id = link.role_id
                     WHERE link.user_account_id = account.id
                     ORDER BY role.code) AS roles,
               ARRAY(SELECT DISTINCT identity.provider::TEXT
                     FROM tlab_user_identity AS identity
                     WHERE identity.user_account_id = account.id
                     ORDER BY identity.provider::TEXT) AS providers
        FROM tlab_user_account AS account
        WHERE ($1::TEXT IS NULL OR account.name ILIKE '%' || $1 || '%' OR account.email ILIKE '%' || $1 || '%')
          AND ($2::TEXT IS NULL OR account.status = $2)
        ORDER BY account.created_at DESC, account.id DESC
        LIMIT $3 OFFSET $4"#,
    )
    .bind(query)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(Paging {
        items: rows
            .into_iter()
            .map(|row| {
                Ok(UserListItem {
                    account: row.account.into_entity()?,
                    roles: row.roles,
                    providers: row.providers,
                })
            })
            .collect::<tlab::Result<Vec<_>>>()?,
        total,
    })
}

pub async fn find_roles_by_user_account_id(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Vec<entity::Role>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT role.id, role.code, role.description FROM tlab_user_role AS link \
         JOIN tlab_role AS role ON role.id = link.role_id \
         WHERE link.user_account_id = $1 ORDER BY role.code",
    )
    .bind(user_account_id)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(rows
        .into_iter()
        .map(|(id, code, description)| entity::Role {
            id,
            code,
            description,
        })
        .collect())
}

pub async fn find_role_ids_by_user_account_id(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Vec<i64>> {
    sqlx::query_scalar(
        "SELECT role_id FROM tlab_user_role WHERE user_account_id = $1 ORDER BY role_id",
    )
    .bind(user_account_id)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)
}

pub async fn add_roles(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    role_ids: &[i64],
) -> tlab::Result<()> {
    sqlx::query(
        "INSERT INTO tlab_user_role (user_account_id, role_id) \
         SELECT $1, id FROM UNNEST($2::BIGINT[]) AS role_id(id) ON CONFLICT DO NOTHING",
    )
    .bind(user_account_id)
    .bind(role_ids)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub async fn remove_roles(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    role_ids: &[i64],
) -> tlab::Result<()> {
    sqlx::query("DELETE FROM tlab_user_role WHERE user_account_id = $1 AND role_id = ANY($2)")
        .bind(user_account_id)
        .bind(role_ids)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct UserIdentityRow {
    id: i64,
    user_account_id: i64,
    provider: String,
    provider_subject: String,
    provider_email: Option<String>,
    created_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

impl UserIdentityRow {
    fn into_entity(self) -> tlab::Result<entity::UserIdentity> {
        let provider = self.provider.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid provider: {}", self.provider).into())
        })?;
        Ok(entity::UserIdentity {
            id: self.id,
            user_account_id: self.user_account_id,
            provider,
            provider_subject: self.provider_subject,
            provider_email: self.provider_email,
            created_at: self.created_at,
            last_login_at: self.last_login_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct UserCredentialRow {
    user_identity_id: i64,
    provider: String,
    password_hash: String,
    password_changed_at: DateTime<Utc>,
}

impl UserCredentialRow {
    fn into_entity(self) -> tlab::Result<entity::UserCredential> {
        let provider = self.provider.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid provider: {}", self.provider).into())
        })?;
        Ok(entity::UserCredential {
            user_identity_id: self.user_identity_id,
            provider,
            password_hash: self.password_hash,
            password_changed_at: self.password_changed_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ManagedLoginUserRow {
    account_id: i64,
    account_name: String,
    account_email: String,
    account_status: String,
    account_created_at: DateTime<Utc>,
    account_updated_at: DateTime<Utc>,
    account_create_by: Option<i64>,
    account_update_by: Option<i64>,
    credential_identity_id: i64,
    password_hash: String,
    password_changed_at: DateTime<Utc>,
}

impl ManagedLoginUserRow {
    fn into_entity(self) -> tlab::Result<entity::ManagedLoginUser> {
        let account = UserAccountRow {
            id: self.account_id,
            name: self.account_name,
            email: self.account_email,
            status: self.account_status,
            created_at: self.account_created_at,
            updated_at: self.account_updated_at,
            create_by: self.account_create_by,
            update_by: self.account_update_by,
        }
        .into_entity()?;
        Ok(entity::ManagedLoginUser {
            account,
            credential: entity::UserCredential {
                user_identity_id: self.credential_identity_id,
                provider: entity::Provider::Managed,
                password_hash: self.password_hash,
                password_changed_at: self.password_changed_at,
            },
        })
    }
}

pub async fn create_user_account(
    context: &mut AppDBCtx<'_>,
    name: &str,
    email: &str,
    status: entity::UserStatus,
) -> tlab::Result<entity::UserAccount> {
    let row = sqlx::query_as::<_, UserAccountRow>(
        r#"INSERT INTO tlab_user_account (name, email, status)
           VALUES ($1, $2, $3)
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
    )
    .bind(name)
    .bind(email)
    .bind(status.as_str())
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.into_entity()
}

pub async fn update_user_account(
    context: &mut AppDBCtx<'_>,
    user_account: &entity::UserAccount,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query_as::<_, UserAccountRow>(
        r#"UPDATE tlab_user_account
           SET name = $1, email = $2, status = $3, update_by = $4, updated_at = CURRENT_TIMESTAMP
           WHERE id = $5
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
    )
    .bind(&user_account.name)
    .bind(&user_account.email)
    .bind(user_account.status.as_str())
    .bind(user_account.update_by)
    .bind(user_account.id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserAccountRow::into_entity).transpose()
}

pub async fn withdraw_user_account(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query_as::<_, UserAccountRow>(
        r#"UPDATE tlab_user_account
           SET status = 'withdrawn', updated_at = CURRENT_TIMESTAMP
           WHERE id = $1
           RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
    )
    .bind(user_account_id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserAccountRow::into_entity).transpose()
}

pub async fn delete_user_account(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query("DELETE FROM tlab_user_account WHERE id = $1")
        .bind(user_account_id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    if result.rows_affected() == 0 {
        return Err(tlab::Error::NotFound("user account".into()));
    }
    Ok(())
}

pub async fn find_user_account(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query_as::<_, UserAccountRow>(
        r#"SELECT id, name, email, status, created_at, updated_at, create_by, update_by
           FROM tlab_user_account WHERE id = $1"#,
    )
    .bind(user_account_id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserAccountRow::into_entity).transpose()
}

pub async fn find_user_account_for_update(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::UserAccount>> {
    let row = sqlx::query_as::<_, UserAccountRow>(
        r#"SELECT id, name, email, status, created_at, updated_at, create_by, update_by
           FROM tlab_user_account WHERE id = $1 FOR UPDATE"#,
    )
    .bind(user_account_id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserAccountRow::into_entity).transpose()
}

pub async fn find_managed_login_user(
    context: &mut AppDBCtx<'_>,
    email: &str,
) -> tlab::Result<Option<entity::ManagedLoginUser>> {
    let row = sqlx::query_as::<_, ManagedLoginUserRow>(
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
    )
    .bind(email)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(ManagedLoginUserRow::into_entity).transpose()
}

pub async fn create_user_identity(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    provider: entity::Provider,
    provider_subject: &str,
    provider_email: Option<&str>,
) -> tlab::Result<entity::UserIdentity> {
    let row = sqlx::query_as::<_, UserIdentityRow>(
        r#"INSERT INTO tlab_user_identity (user_account_id, provider, provider_subject, provider_email)
           VALUES ($1, $2, $3, $4)
           RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
    )
    .bind(user_account_id)
    .bind(provider.as_str())
    .bind(provider_subject)
    .bind(provider_email)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.into_entity()
}

pub async fn find_user_identity(
    context: &mut AppDBCtx<'_>,
    provider: entity::Provider,
    provider_subject: &str,
) -> tlab::Result<Option<entity::UserIdentity>> {
    let row = sqlx::query_as::<_, UserIdentityRow>(
        r#"SELECT id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at
           FROM tlab_user_identity WHERE provider = $1 AND provider_subject = $2"#,
    )
    .bind(provider.as_str())
    .bind(provider_subject)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserIdentityRow::into_entity).transpose()
}

pub async fn update_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity: &entity::UserIdentity,
) -> tlab::Result<Option<entity::UserIdentity>> {
    let row = sqlx::query_as::<_, UserIdentityRow>(
        r#"UPDATE tlab_user_identity
           SET provider_email = $1, last_login_at = $2
           WHERE id = $3
           RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
    )
    .bind(user_identity.provider_email.as_deref())
    .bind(user_identity.last_login_at)
    .bind(user_identity.id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserIdentityRow::into_entity).transpose()
}

pub async fn delete_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query(r#"DELETE FROM tlab_user_identity WHERE id = $1"#)
        .bind(user_identity_id)
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
    let row = sqlx::query_as::<_, UserCredentialRow>(
        r#"INSERT INTO tlab_user_credential (user_identity_id, provider, password_hash)
           VALUES ($1, $2, $3)
           RETURNING user_identity_id, provider, password_hash, password_changed_at"#,
    )
    .bind(user_identity_id)
    .bind(provider.as_str())
    .bind(password_hash)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.into_entity()
}

pub async fn update_user_credential(
    context: &mut AppDBCtx<'_>,
    user_credential: &entity::UserCredential,
) -> tlab::Result<Option<entity::UserCredential>> {
    let row = sqlx::query_as::<_, UserCredentialRow>(
        r#"UPDATE tlab_user_credential
           SET password_hash = $1, password_changed_at = CURRENT_TIMESTAMP
           WHERE user_identity_id = $2 AND provider = $3
           RETURNING user_identity_id, provider, password_hash, password_changed_at"#,
    )
    .bind(&user_credential.password_hash)
    .bind(user_credential.user_identity_id)
    .bind(user_credential.provider.as_str())
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    row.map(UserCredentialRow::into_entity).transpose()
}

pub async fn delete_user_credential(
    context: &mut AppDBCtx<'_>,
    user_identity_id: i64,
) -> tlab::Result<()> {
    let result = sqlx::query(r#"DELETE FROM tlab_user_credential WHERE user_identity_id = $1"#)
        .bind(user_identity_id)
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
    use crate::test_db;

    #[tokio::test]
    async fn finds_assigned_roles() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Alice', $1) RETURNING id",
        )
        .bind(test_db::unique_email("rbac"))
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        let sales_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'sales'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();

        add_roles(&mut tx.context(), user_id, &[admin_role_id, sales_role_id])
            .await
            .unwrap();
        assert_eq!(
            find_role_ids_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap(),
            [admin_role_id, sales_role_id]
        );
        assert_eq!(
            find_roles_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap()
                .into_iter()
                .map(|role| role.code)
                .collect::<Vec<_>>(),
            ["admin", "sales"]
        );
        remove_roles(&mut tx.context(), user_id, &[sales_role_id])
            .await
            .unwrap();
        assert_eq!(
            find_role_ids_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap(),
            [admin_role_id]
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn runs_user_repository_cud_with_postgres() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let mut context = tx.context();

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
        delete_user_account(&mut context, account.id).await.unwrap();

        drop(context);
        tx.rollback().await.unwrap();
    }
}
