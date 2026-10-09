#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Suspended,
    Withdrawn,
}

impl UserStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Withdrawn => "withdrawn",
        }
    }
}

impl std::str::FromStr for UserStatus {
    type Err = ();

    fn from_str(status: &str) -> Result<Self, Self::Err> {
        match status {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "withdrawn" => Ok(Self::Withdrawn),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Managed,
    Google,
}

impl Provider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Google => "google",
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = ();

    fn from_str(provider: &str) -> Result<Self, Self::Err> {
        match provider {
            "managed" => Ok(Self::Managed),
            "google" => Ok(Self::Google),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserAccount {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub create_by: Option<i64>,
    pub update_by: Option<i64>,
}

impl UserAccount {
    pub fn new(name: String, email: String, status: UserStatus) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: -1,
            name,
            email,
            status,
            created_at: now,
            updated_at: now,
            create_by: None,
            update_by: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Permission {
    pub id: i64,
    pub code: String,
    pub description: Option<String>,
}

impl Permission {
    pub fn new(code: String, description: Option<String>) -> Self {
        Self {
            id: -1,
            code,
            description,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Role {
    pub id: i64,
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserIdentity {
    pub id: i64,
    pub user_account_id: i64,
    pub provider: Provider,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl UserIdentity {
    pub fn new(
        user_account_id: i64,
        provider: Provider,
        provider_subject: String,
        provider_email: Option<String>,
    ) -> Self {
        Self {
            id: -1,
            user_account_id,
            provider,
            provider_subject,
            provider_email,
            created_at: chrono::Utc::now(),
            last_login_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserCredential {
    pub user_identity_id: i64,
    pub provider: Provider,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}

impl UserCredential {
    pub fn new(user_identity_id: i64, provider: Provider, password_hash: String) -> Self {
        Self {
            user_identity_id,
            provider,
            password_hash,
            password_changed_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub session_id: uuid::Uuid,
    pub user_account_id: i64,
    pub token_hash: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub account: UserAccount,
    pub identities: Vec<UserIdentity>,
    pub credentials: Vec<UserCredential>,
    pub roles: Vec<Role>,
}
