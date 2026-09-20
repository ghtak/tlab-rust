#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserAccount {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub create_by: Option<i64>,
    pub update_by: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserIdentity {
    pub id: i64,
    pub user_account_id: i64,
    pub provider: String,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCredential {
    pub user_identity_id: i64,
    pub provider: String,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}
