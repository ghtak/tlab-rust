pub(crate) const AUTH_MIGRATIONS: [(&str, &str); 4] = [
    (
        "001_create_user_account.sql",
        include_str!("auth/migrations/001_create_user_account.sql"),
    ),
    (
        "002_create_user_identity_and_credential.sql",
        include_str!("auth/migrations/002_create_user_identity_and_credential.sql"),
    ),
    (
        "003_create_refresh_token.sql",
        include_str!("auth/migrations/003_create_refresh_token.sql"),
    ),
    (
        "004_create_rbac.sql",
        include_str!("auth/migrations/004_create_rbac.sql"),
    ),
];
