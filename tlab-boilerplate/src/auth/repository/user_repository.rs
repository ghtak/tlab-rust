#![allow(dead_code)]

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use tlab::{paging::Paging, sqlxdb};

use crate::{
    app_container::{AppDBCtx, AppQueryBuilder},
    auth::entity,
};

#[derive(sqlx::FromRow, Clone, Debug)]
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

#[derive(sqlx::FromRow, serde::Deserialize)]
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

#[derive(sqlx::FromRow, serde::Deserialize)]
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

#[derive(sqlx::FromRow, serde::Deserialize)]
struct RoleRow {
    id: i64,
    code: String,
    description: Option<String>,
}

impl RoleRow {
    fn into_entity(self) -> entity::Role {
        entity::Role {
            id: self.id,
            code: self.code,
            description: self.description,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    #[sqlx(flatten)]
    account: UserAccountRow,
    identities: sqlx::types::Json<Vec<UserIdentityRow>>,
    credentials: sqlx::types::Json<Vec<UserCredentialRow>>,
    roles: sqlx::types::Json<Vec<RoleRow>>,
}

impl UserRow {
    fn into_entity(self) -> tlab::Result<entity::User> {
        let account = self.account.into_entity()?;
        let identities = self
            .identities
            .0
            .into_iter()
            .map(|i| i.into_entity())
            .collect::<tlab::Result<Vec<_>>>()?;
        let credentials = self
            .credentials
            .0
            .into_iter()
            .map(|c| c.into_entity())
            .collect::<tlab::Result<Vec<_>>>()?;
        let roles = self
            .roles
            .0
            .into_iter()
            .map(|r| r.into_entity())
            .collect::<Vec<_>>();

        Ok(entity::User {
            account,
            identities,
            credentials,
            roles,
        })
    }
}

pub struct UserSearchCriteria<'a> {
    pub query: Option<&'a str>,
    pub status: Option<entity::UserStatus>,
    pub identity_provider: Option<entity::Provider>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn search(
    context: &mut AppDBCtx<'_>,
    criteria: &UserSearchCriteria<'_>,
) -> tlab::Result<Paging<entity::User>> {
    let mut builder = AppQueryBuilder::new(r#"SELECT COUNT(*) FROM tlab_user_account WHERE 1=1 "#);

    if let Some(query) = criteria.query {
        let query = format!("%{}%", query);
        builder.push(" AND (name ILIKE ");
        builder.push_bind(query.as_str());
        builder.push(" OR email ILIKE ");
        builder.push_bind(query.as_str());
        builder.push(") ");
    }

    if let Some(status) = &criteria.status {
        builder.push(" AND status = ");
        builder.push_bind(status.as_str());
    }

    let total: i64 = builder
        .build_query_scalar()
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    let mut builder = AppQueryBuilder::new(
        r#"
        SELECT
            account.id,
            account.name,
            account.email,
            account.status,
            account.created_at,
            account.updated_at,
            account.create_by,
            account.update_by,
        "#,
    );

    // 1. identities 서브쿼리
    builder.push(
        r#"
            COALESCE((
                SELECT json_agg(
                    json_build_object(
                        'id', identity.id,
                        'user_account_id', identity.user_account_id,
                        'provider', identity.provider,
                        'provider_subject', identity.provider_subject,
                        'provider_email', identity.provider_email,
                        'created_at', identity.created_at,
                        'last_login_at', identity.last_login_at
                    ) ORDER BY identity.created_at, identity.id
                )
                FROM tlab_user_identity AS identity
                WHERE identity.user_account_id = account.id
        "#,
    );

    if let Some(provider) = &criteria.identity_provider {
        builder.push(" AND identity.provider = ");
        builder.push_bind(provider.as_str());
    }

    builder.push(
        r#"
            ), '[]'::json ) AS identities,
        "#,
    );

    // 2. credentials 서브쿼리
    builder.push(
        r#"
            COALESCE((
                SELECT json_agg(
                    json_build_object(
                        'user_identity_id', credential.user_identity_id,
                        'provider', credential.provider,
                        'password_hash', credential.password_hash,
                        'password_changed_at', credential.password_changed_at
                    )
                )
                FROM tlab_user_credential AS credential
                JOIN tlab_user_identity AS identity ON identity.id = credential.user_identity_id
                WHERE identity.user_account_id = account.id
            ), '[]'::json ) AS credentials ,
        "#,
    );

    builder.push(
        r#"
            COALESCE((
                    SELECT json_agg(
                        json_build_object(
                            'id', role.id,
                            'code', role.code,
                            'description', role.description
                        )
                    )
                    FROM tlab_user_role AS user_role
                    JOIN tlab_role AS role ON role.id = user_role.role_id
                    WHERE user_role.user_account_id = account.id
                ), '[]'::json ) AS roles
        "#,
    );

    builder.push(
        r#"
        FROM tlab_user_account AS account
        WHERE 1=1
        "#,
    );

    // 특정 provider를 가진 account만 조회되도록 EXISTS 추가
    if let Some(provider) = &criteria.identity_provider {
        builder.push(
            r#"
            AND EXISTS (
                SELECT 1 FROM tlab_user_identity AS id_check
                WHERE id_check.user_account_id = account.id
                  AND id_check.provider =
            "#,
        );
        builder.push_bind(provider.as_str());
        builder.push(" ) ");
    }

    // 3. 메인 account 조건
    if let Some(query) = &criteria.query {
        let pattern = format!("%{}%", query);
        builder.push(" AND (account.name ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR account.email ILIKE ");
        builder.push_bind(pattern);
        builder.push(") ");
    }

    if let Some(status) = &criteria.status {
        builder.push(" AND account.status = ");
        builder.push_bind(status.as_str());
    }

    builder.push(r#" ORDER BY account.created_at DESC, account.id DESC LIMIT "#);
    builder.push_bind(criteria.limit);
    builder.push(" OFFSET ");
    builder.push_bind(criteria.offset);

    // 4. Query 실행
    let rows: Vec<UserRow> = builder
        .build_query_as::<UserRow>()
        .fetch_all(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    // 5. Convert
    let items: Vec<entity::User> = rows
        .into_iter()
        .map(|row| -> tlab::Result<entity::User> { row.into_entity() })
        .collect::<tlab::Result<Vec<_>>>()?;
    Ok(Paging { items, total })
}

#[derive(Debug, sqlx::FromRow)]
struct UserFlatRow {
    // Account (Flatten)
    #[sqlx(flatten)]
    account: UserAccountRow,

    // Identity (LEFT JOIN 이므로 Nullable)
    identity_id: Option<i64>,
    identity_provider: Option<String>,
    identity_provider_subject: Option<String>,
    identity_provider_email: Option<String>,
    identity_created_at: Option<chrono::DateTime<chrono::Utc>>,
    identity_last_login_at: Option<chrono::DateTime<chrono::Utc>>,

    // Credential (LEFT JOIN 이므로 Nullable)
    credential_user_identity_id: Option<i64>,
    credential_provider: Option<String>,
    credential_password_hash: Option<String>,
    credential_password_changed_at: Option<chrono::DateTime<chrono::Utc>>,

    // Role (LEFT JOIN 이므로 Nullable)
    role_id: Option<i64>,
    role_code: Option<String>,
    role_description: Option<String>,
}

const USER_FLAT_SELECT: &str = r#"
        SELECT
            account.id, account.name, account.email, account.status,
            account.created_at, account.updated_at, account.create_by, account.update_by,

            identity.id AS identity_id,
            identity.provider AS identity_provider,
            identity.provider_subject AS identity_provider_subject,
            identity.provider_email AS identity_provider_email,
            identity.created_at AS identity_created_at,
            identity.last_login_at AS identity_last_login_at,

            credential.user_identity_id AS credential_user_identity_id,
            credential.provider AS credential_provider,
            credential.password_hash AS credential_password_hash,
            credential.password_changed_at AS credential_password_changed_at,

            role.id AS role_id,
            role.code AS role_code,
            role.description AS role_description

        FROM tlab_user_account AS account
        LEFT JOIN tlab_user_identity AS identity
               ON identity.user_account_id = account.id
        LEFT JOIN tlab_user_credential AS credential
               ON credential.user_identity_id = identity.id
        LEFT JOIN tlab_user_role AS user_role
               ON user_role.user_account_id = account.id
        LEFT JOIN tlab_role AS role
               ON role.id = user_role.role_id
"#;

fn assemble_user(rows: Vec<UserFlatRow>) -> tlab::Result<Option<entity::User>> {
    if rows.is_empty() {
        return Ok(None);
    }

    // 1. Account Entity 추출 (첫 번째 행)
    let account = rows[0].account.clone().into_entity()?;

    let mut identities = Vec::new();
    let mut credentials = Vec::new();
    let mut roles = Vec::new();

    // 중복 추가 방지용 Set (N x M Join에 따른 중복 제거)
    let mut seen_identity_ids = HashSet::new();
    let mut seen_credential_keys = HashSet::new();
    let mut seen_role_ids = HashSet::new();

    // 2. Flat Rows 순회하며 Rust 메모리 상에서 조합
    for row in rows {
        // Identity 수집
        if let Some(identity_id) = row.identity_id {
            if seen_identity_ids.insert(identity_id) {
                identities.push(
                    UserIdentityRow {
                        id: identity_id,
                        user_account_id: account.id,
                        provider: row.identity_provider.clone().unwrap(),
                        provider_subject: row.identity_provider_subject.clone().unwrap(),
                        provider_email: row.identity_provider_email.clone(),
                        created_at: row.identity_created_at.unwrap(),
                        last_login_at: row.identity_last_login_at,
                    }
                    .into_entity()?,
                );
            }
        }

        // Credential 수집
        if let (Some(identity_id), Some(provider)) = (
            row.credential_user_identity_id,
            row.credential_provider.as_ref(),
        ) {
            let key = (identity_id, provider.clone());
            if seen_credential_keys.insert(key) {
                credentials.push(
                    UserCredentialRow {
                        user_identity_id: identity_id,
                        provider: provider.clone(),
                        password_hash: row.credential_password_hash.clone().unwrap(),
                        password_changed_at: row.credential_password_changed_at.unwrap(),
                    }
                    .into_entity()?,
                );
            }
        }

        if let Some(role_id) = row.role_id {
            if seen_role_ids.insert(role_id) {
                // Role 수집
                let role = entity::Role {
                    id: role_id,
                    code: row.role_code.clone().unwrap(),
                    description: row.role_description.clone(),
                };
                roles.push(role);
            }
        }
    }

    Ok(Some(entity::User {
        account,
        identities,
        credentials,
        roles,
    }))
}

pub async fn find_user(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::User>> {
    let mut builder = AppQueryBuilder::new(USER_FLAT_SELECT);
    builder.push(" WHERE account.id = ");
    builder.push_bind(user_account_id);
    let rows = builder
        .build_query_as::<UserFlatRow>()
        .fetch_all(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    assemble_user(rows)
}

pub async fn find_user_by_email(
    context: &mut AppDBCtx<'_>,
    email: &str,
) -> tlab::Result<Option<entity::User>> {
    let mut builder = AppQueryBuilder::new(USER_FLAT_SELECT);
    builder.push(" WHERE account.email = ");
    builder.push_bind(email);
    let rows = builder
        .build_query_as::<UserFlatRow>()
        .fetch_all(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    assemble_user(rows)
}


pub async fn find_all_roles(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Vec<entity::Role>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT role.id, role.code, role.description FROM tlab_user_role AS user_role  \
         JOIN tlab_role AS role ON role.id = user_role .role_id \
         WHERE user_role .user_account_id = $1 ORDER BY role.code",
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

pub async fn find_all_role_ids(
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

pub async fn replace_roles(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    role_ids: &[i64],
) -> tlab::Result<()> {
    sqlx::query("DELETE FROM tlab_user_role WHERE user_account_id = $1")
        .bind(user_account_id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    if !role_ids.is_empty() {
        sqlx::query(
            "INSERT INTO tlab_user_role (user_account_id, role_id) \
             SELECT $1, id FROM UNNEST($2::BIGINT[]) AS role_id(id)",
        )
        .bind(user_account_id)
        .bind(role_ids)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    }
    Ok(())
}


pub async fn save_user_account(
    context: &mut AppDBCtx<'_>,
    user_account: &entity::UserAccount,
) -> tlab::Result<entity::UserAccount> {
    let row = if user_account.id == -1 {
        sqlx::query_as::<_, UserAccountRow>(
            r#"INSERT INTO tlab_user_account (name, email, status, create_by, update_by)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, name, email, status, created_at, updated_at, create_by, update_by"#,
        )
        .bind(&user_account.name)
        .bind(&user_account.email)
        .bind(user_account.status.as_str())
        .bind(user_account.create_by)
        .bind(user_account.update_by)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
    } else {
        sqlx::query_as::<_, UserAccountRow>(
            r#"UPDATE tlab_user_account
               SET name = $1, email = $2, status = $3,
                   update_by = $4, updated_at = CURRENT_TIMESTAMP
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
        .map_err(sqlxdb::postgres::map_error)?
        .ok_or(tlab::Error::NotFound("user account".into()))?
    };

    row.into_entity()
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

pub async fn save_user_identity(
    context: &mut AppDBCtx<'_>,
    user_identity: &entity::UserIdentity,
) -> tlab::Result<entity::UserIdentity> {
    let row = if user_identity.id == -1 {
        sqlx::query_as::<_, UserIdentityRow>(
            r#"INSERT INTO tlab_user_identity (user_account_id, provider, provider_subject, provider_email)
               VALUES ($1, $2, $3, $4)
               RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
        )
        .bind(user_identity.user_account_id)
        .bind(user_identity.provider.as_str())
        .bind(&user_identity.provider_subject)
        .bind(&user_identity.provider_email)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
    } else {
        sqlx::query_as::<_, UserIdentityRow>(
            r#"UPDATE tlab_user_identity
               SET provider_email = $1, last_login_at = $2
               WHERE id = $3
               RETURNING id, user_account_id, provider, provider_subject, provider_email, created_at, last_login_at"#,
        )
        .bind(&user_identity.provider_email)
        .bind(user_identity.last_login_at)
        .bind(user_identity.id)
        .fetch_optional(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
        .ok_or(tlab::Error::NotFound("user identity".into()))?
    };

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

pub async fn save_user_credential(
    context: &mut AppDBCtx<'_>,
    user_credential: &entity::UserCredential,
) -> tlab::Result<entity::UserCredential> {
    let row = sqlx::query_as::<_, UserCredentialRow>(
        r#"INSERT INTO tlab_user_credential (user_identity_id, provider, password_hash)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_identity_id) DO UPDATE
           SET password_hash = EXCLUDED.password_hash, password_changed_at = CURRENT_TIMESTAMP
           WHERE tlab_user_credential.provider = EXCLUDED.provider
           RETURNING user_identity_id, provider, password_hash, password_changed_at"#,
    )
    .bind(user_credential.user_identity_id)
    .bind(user_credential.provider.as_str())
    .bind(&user_credential.password_hash)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?
    .ok_or(tlab::Error::NotFound("user credential".into()))?;

    row.into_entity()
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

        replace_roles(&mut tx.context(), user_id, &[admin_role_id, sales_role_id])
            .await
            .unwrap();
        assert_eq!(
            find_all_role_ids(&mut tx.context(), user_id)
                .await
                .unwrap(),
            [admin_role_id, sales_role_id]
        );
        assert_eq!(
            find_all_roles(&mut tx.context(), user_id)
                .await
                .unwrap()
                .into_iter()
                .map(|role| role.code)
                .collect::<Vec<_>>(),
            ["admin", "sales"]
        );
        replace_roles(&mut tx.context(), user_id, &[admin_role_id])
            .await
            .unwrap();
        assert_eq!(
            find_all_role_ids(&mut tx.context(), user_id)
                .await
                .unwrap(),
            [admin_role_id]
        );
        replace_roles(&mut tx.context(), user_id, &[])
            .await
            .unwrap();
        assert!(
            find_all_role_ids(&mut tx.context(), user_id)
                .await
                .unwrap()
                .is_empty()
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
        let mut account = save_user_account(
            &mut context,
            &entity::UserAccount::new("Alice".into(), email.clone(), entity::UserStatus::Active),
        )
        .await
        .unwrap();
        assert!(account.id > 0);
        let created_at = account.created_at;
        let id = account.id;
        account.name = "Alice Updated".into();
        let mut account = save_user_account(&mut context, &account).await.unwrap();
        assert_eq!(account.id, id);
        assert_eq!(account.created_at, created_at);
        assert_eq!(account.name, "Alice Updated");

        account.status = entity::UserStatus::Withdrawn;
        let account = save_user_account(&mut context, &account).await.unwrap();
        assert_eq!(account.status, entity::UserStatus::Withdrawn);
        let mut missing_account = account.clone();
        missing_account.id = i64::MAX;
        assert!(matches!(
            save_user_account(&mut context, &missing_account).await,
            Err(tlab::Error::NotFound(_))
        ));

        let mut identity = save_user_identity(
            &mut context,
            &entity::UserIdentity::new(
                account.id,
                entity::Provider::Managed,
                suffix.clone(),
                Some("alice@example.com".into()),
            ),
        )
        .await
        .unwrap();
        assert!(identity.id > 0);
        let identity_id = identity.id;
        let identity_created_at = identity.created_at;
        identity.provider_email = Some("updated@example.com".into());
        let identity = save_user_identity(&mut context, &identity).await.unwrap();
        assert_eq!(identity.id, identity_id);
        assert_eq!(identity.created_at, identity_created_at);
        assert_eq!(
            identity.provider_email.as_deref(),
            Some("updated@example.com")
        );
        let mut missing_identity = identity.clone();
        missing_identity.id = i64::MAX;
        assert!(matches!(
            save_user_identity(&mut context, &missing_identity).await,
            Err(tlab::Error::NotFound(_))
        ));

        let mut credential = save_user_credential(
            &mut context,
            &entity::UserCredential::new(identity.id, identity.provider, "initial-hash".into()),
        )
        .await
        .unwrap();
        credential.password_hash = "updated-hash".into();
        let credential = save_user_credential(&mut context, &credential).await.unwrap();
        assert_eq!(credential.password_hash, "updated-hash");

        let user = find_user_by_email(&mut context, &email)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.account.id, account.id);
        assert_eq!(user.identities.len(), 1);
        assert_eq!(user.credentials[0].password_hash, "updated-hash");
        assert_eq!(
            find_user(&mut context, account.id)
                .await
                .unwrap()
                .unwrap()
                .account
                .email,
            email
        );
        assert!(find_user_by_email(&mut context, &suffix).await.unwrap().is_none());
        assert!(
            find_user_by_email(&mut context, "missing@example.com")
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
