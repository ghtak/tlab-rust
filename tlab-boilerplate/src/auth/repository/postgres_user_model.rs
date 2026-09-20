use crate::auth::entity;

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserAccount {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub create_by: Option<i64>,
    pub update_by: Option<i64>,
}

impl TryFrom<PostgresUserAccount> for entity::UserAccount {
    type Error = tlab::Error;

    fn try_from(model: PostgresUserAccount) -> Result<Self, Self::Error> {
        let status = model.status.parse().map_err(|_| {
            tlab::Error::IllegalState(format!("invalid user status: {}", model.status).into())
        })?;
        Ok(Self {
            id: model.id,
            name: model.name,
            email: model.email,
            status,
            created_at: model.created_at,
            updated_at: model.updated_at,
            create_by: model.create_by,
            update_by: model.update_by,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserIdentity {
    pub id: i64,
    pub user_account_id: i64,
    pub provider: String,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PostgresUserIdentity> for entity::UserIdentity {
    fn from(model: PostgresUserIdentity) -> Self {
        Self {
            id: model.id,
            user_account_id: model.user_account_id,
            provider: model.provider,
            provider_subject: model.provider_subject,
            provider_email: model.provider_email,
            created_at: model.created_at,
            last_login_at: model.last_login_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct PostgresUserCredential {
    pub user_identity_id: i64,
    pub provider: String,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}

impl From<PostgresUserCredential> for entity::UserCredential {
    fn from(model: PostgresUserCredential) -> Self {
        Self {
            user_identity_id: model.user_identity_id,
            provider: model.provider,
            password_hash: model.password_hash,
            password_changed_at: model.password_changed_at,
        }
    }
}
