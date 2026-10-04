use crate::{
    app_container::{AppDB, AppDBCtx},
    auth::{entity, repository::user_repository},
};

const ADMIN_EMAIL: &str = "admin@localhost";

const AUTH_MIGRATIONS: [(&str, &str); 3] = [
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
];

pub async fn migrate(
    database: &AppDB,
    password_hasher: &dyn tlab::hash::PasswordHasher,
) -> tlab::Result<()> {
    let mut tx = database.tx().await?;

    for (name, sql) in AUTH_MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(tx.context().backend())
            .await
            .map_err(|source| tlab::Error::DbDriver {
                source: source.into(),
            })?;
        tracing::info!(migration = name, "Migration applied");
    }

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
        let account = user_repository::create_user_account(
            context,
            "admin",
            ADMIN_EMAIL,
            entity::UserStatus::Active,
        )
        .await?;
        let identity = user_repository::create_user_identity(
            context,
            account.id,
            entity::Provider::Managed,
            ADMIN_EMAIL,
            Some(ADMIN_EMAIL),
        )
        .await?;
        user_repository::create_user_credential(
            context,
            identity.id,
            entity::Provider::Managed,
            &password_hash,
        )
        .await?;
        tracing::info!(email = ADMIN_EMAIL, "Managed admin user created");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tlab::hash::{Argon2Config, Argon2PasswordHasher, PasswordHasher};

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn creates_admin_once_with_a_verifiable_password() {
        let database = AppDB::new(&tlab::sqlxdb::Config {
            url: "postgres://tlab:tlab@localhost:25432/tlab".into(),
            max_connections: 1,
        })
        .await
        .unwrap();
        let mut tx = database.tx().await.unwrap();
        let schema = format!(
            "admin_seed_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        );

        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(tx.context().backend())
            .await
            .unwrap();
        sqlx::query_scalar::<_, String>("SELECT set_config('search_path', $1, true)")
            .bind(&schema)
            .fetch_one(tx.context().backend())
            .await
            .unwrap();
        for (_, sql) in AUTH_MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(tx.context().backend())
                .await
                .unwrap();
        }

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

        create_admin_user_if_missing(&mut tx.context(), &password_hasher)
            .await
            .unwrap();

        tx.rollback().await.unwrap();
    }
}
