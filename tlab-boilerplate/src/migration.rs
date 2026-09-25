use std::sync::Arc;

use crate::app_container::AppContainer;

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

pub async fn migrate(container: Arc<AppContainer>) -> tlab::Result<()> {
    let mut tx = container.database.tx().await?;

    for (name, sql) in AUTH_MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(tx.context().backend())
            .await
            .map_err(|source| tlab::Error::DbDriver {
                source: source.into(),
            })?;
        tracing::info!(migration = name, "Migration applied");
    }

    tx.commit().await?;
    Ok(())
}
