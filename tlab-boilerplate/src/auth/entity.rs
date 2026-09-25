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
}

impl Provider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = ();

    fn from_str(provider: &str) -> Result<Self, Self::Err> {
        match provider {
            "managed" => Ok(Self::Managed),
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

#[derive(Debug, Clone)]
pub struct UserCredential {
    pub user_identity_id: i64,
    pub provider: Provider,
    pub password_hash: String,
    pub password_changed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ManagedLoginUser {
    pub account: UserAccount,
    pub credential: UserCredential,
}
