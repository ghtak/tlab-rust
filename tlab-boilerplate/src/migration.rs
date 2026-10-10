use crate::{
    app_container::{AppDB, AppDBCtx},
    auth::{entity, repository::user_repository},
};

const ADMIN_EMAIL: &str = "admin@localhost";

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

pub async fn migrate(database: &AppDB) -> tlab::Result<()> {
    let mut tx = database.tx().await?;
    for (name, sql) in AUTH_MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(tx.context().backend())
            .await
            .map_err(tlab::sqlxdb::postgres::map_error)?;
        tracing::info!(migration = name, "Migration applied");
    }
    tx.commit().await?;
    Ok(())
}

pub async fn initialize_admin(
    database: &AppDB,
    password_hasher: &dyn tlab::hash::PasswordHasher,
) -> tlab::Result<()> {
    let mut tx = database.tx().await?;

    create_admin_user_if_missing(&mut tx.context(), password_hasher).await?;

    tx.commit().await?;
    Ok(())
}

async fn create_admin_user_if_missing(
    context: &mut AppDBCtx<'_>,
    password_hasher: &dyn tlab::hash::PasswordHasher,
) -> tlab::Result<()> {
    let admin_exists =
        sqlx::query_scalar::<_, i64>("SELECT id FROM tlab_user_account WHERE email = $1")
            .bind(ADMIN_EMAIL)
            .fetch_optional(context.backend())
            .await
            .map_err(tlab::sqlxdb::postgres::map_error)?
            .is_some();

    if !admin_exists {
        let password_hash = password_hasher.hash("passwd")?;
        let account = user_repository::save_user_account(
            context,
            &entity::UserAccount::new(
                "admin".into(),
                ADMIN_EMAIL.into(),
                entity::UserStatus::Active,
            ),
        )
        .await?;
        let identity = user_repository::save_user_identity(
            context,
            &entity::UserIdentity::new(
                account.id,
                entity::Provider::Managed,
                ADMIN_EMAIL.into(),
                Some(ADMIN_EMAIL.into()),
            ),
        )
        .await?;
        user_repository::save_user_credential(
            context,
            &entity::UserCredential::new(identity.id, entity::Provider::Managed, password_hash),
        )
        .await?;
        tracing::info!(email = ADMIN_EMAIL, "Managed admin user created");
    }

    sqlx::query(
        "INSERT INTO tlab_user_role (user_account_id, role_id) \
         SELECT account.id, role.id FROM tlab_user_account AS account \
         JOIN tlab_role AS role ON role.code = 'admin' WHERE account.email = $1 \
         ON CONFLICT DO NOTHING",
    )
    .bind(ADMIN_EMAIL)
    .execute(context.backend())
    .await
    .map_err(tlab::sqlxdb::postgres::map_error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;
    use tlab::hash::{Argon2Config, Argon2PasswordHasher, PasswordHasher};

    #[tokio::test]
    async fn creates_admin_once_with_a_verifiable_password() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();

        let password_hasher = Argon2PasswordHasher::new(&Argon2Config {
            memory_cost_kib: 19456,
            time_cost: 2,
            parallelism: 1,
            output_len: Some(32),
        })
        .unwrap();
        create_admin_user_if_missing(&mut tx.context(), &password_hasher)
            .await
            .unwrap();

        let account: (i64, String, String, String) = sqlx::query_as(
            "SELECT account.id, account.name, account.email, credential.password_hash \
             FROM tlab_user_account AS account \
             JOIN tlab_user_identity AS identity ON identity.user_account_id = account.id \
             JOIN tlab_user_credential AS credential ON credential.user_identity_id = identity.id \
             WHERE account.email = $1 AND identity.provider = 'managed'",
        )
        .bind(ADMIN_EMAIL)
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        assert_eq!(account.1, "admin");
        assert_eq!(account.2, ADMIN_EMAIL);
        password_hasher.verify("passwd", &account.3).unwrap();

        let permissions: Vec<String> = sqlx::query_scalar(
            "SELECT permission.code FROM tlab_user_role AS user_role \
             JOIN tlab_role_permission AS role_permission ON role_permission.role_id = user_role.role_id \
             JOIN tlab_permission AS permission ON permission.id = role_permission.permission_id \
             WHERE user_role.user_account_id = $1 ORDER BY permission.code",
        )
        .bind(account.0)
        .fetch_all(tx.context().backend())
        .await
        .unwrap();
        for code in ["access:manage", "file:manage"] {
            assert!(permissions.iter().any(|permission| permission == code));
        }

        create_admin_user_if_missing(&mut tx.context(), &password_hasher)
            .await
            .unwrap();

        let role_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tlab_user_role AS user_role \
             JOIN tlab_role AS role ON role.id = user_role.role_id \
             WHERE user_role.user_account_id = $1 AND role.code = 'admin'",
        )
        .bind(account.0)
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        assert_eq!(role_count, 1);

        tx.rollback().await.unwrap();
    }
}
