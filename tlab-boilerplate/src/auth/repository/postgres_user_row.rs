use crate::auth::entity;

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserAccountRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub create_by: Option<i64>,
    pub update_by: Option<i64>,
}

impl TryFrom<PostgresUserAccountRow> for entity::UserAccount {
    type Error = tlab::Error;

    fn try_from(row: PostgresUserAccountRow) -> Result<Self, Self::Error> {
        let status = row.status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", row.status).into())
        })?;
        Ok(Self {
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
}

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserIdentityRow {
    pub id: i64,
    pub user_account_id: i64,
    pub provider: String,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PostgresUserIdentityRow> for entity::UserIdentity {
    fn from(row: PostgresUserIdentityRow) -> Self {
        Self {
            id: row.id,
            user_account_id: row.user_account_id,
            provider: row.provider,
            provider_subject: row.provider_subject,
            provider_email: row.provider_email,
            created_at: row.created_at,
            last_login_at: row.last_login_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserCredentialRow {
    pub user_identity_id: i64,
    pub provider: String,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}

impl From<PostgresUserCredentialRow> for entity::UserCredential {
    fn from(row: PostgresUserCredentialRow) -> Self {
        Self {
            user_identity_id: row.user_identity_id,
            provider: row.provider,
            password_hash: row.password_hash,
            password_changed_at: row.password_changed_at,
        }
    }
}
