#[derive(Debug, Clone)]
pub struct UserAccount {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub create_by: Option<u64>,
    pub update_by: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UserIdentity {
    pub id: u64,
    pub user_account_id: u64,
    pub provider: String,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct UserCredential {
    pub user_identity_id: u64,
    pub provider: String,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}
