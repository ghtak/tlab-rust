use chrono::{DateTime, Utc};
use tlab::sqlxdb;
use uuid::Uuid;

use crate::{app_container::AppDBCtx, auth::entity};

pub async fn save(
    context: &mut AppDBCtx<'_>,
    session_id: Uuid,
    user_account_id: i64,
    token_hash: &[u8; 32],
    expires_at: DateTime<Utc>,
) -> tlab::Result<()> {
    sqlx::query(
        r#"INSERT INTO tlab_refresh_token (session_id, user_account_id, token_hash, expires_at)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(session_id)
    .bind(user_account_id)
    .bind(token_hash.as_slice())
    .bind(expires_at)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub async fn find_all_by_user_account_id(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Vec<entity::RefreshToken>> {
    let rows = sqlx::query_as::<_, (Uuid, i64, Vec<u8>, DateTime<Utc>, DateTime<Utc>)>(
        r#"SELECT session_id, user_account_id, token_hash, created_at, expires_at
           FROM tlab_refresh_token WHERE user_account_id = $1 ORDER BY created_at, session_id"#,
    )
    .bind(user_account_id)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(rows
        .into_iter()
        .map(
            |(session_id, user_account_id, token_hash, created_at, expires_at)| {
                entity::RefreshToken {
                    session_id,
                    user_account_id,
                    token_hash,
                    created_at,
                    expires_at,
                }
            },
        )
        .collect())
}

pub async fn delete(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    session_id: Uuid,
) -> tlab::Result<bool> {
    let result = sqlx::query(
        r#"DELETE FROM tlab_refresh_token WHERE user_account_id = $1 AND session_id = $2"#,
    )
    .bind(user_account_id)
    .bind(session_id)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(result.rows_affected() != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migration_sql::AUTH_MIGRATIONS, test_db};

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn stores_multiple_sessions_and_deletes_only_one() {
        let database = test_db::connect().await;
        let mut tx =
            test_db::isolated_tx(&database, &[AUTH_MIGRATIONS[0].1, AUTH_MIGRATIONS[2].1]).await;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Alice', 'alice@example.com') RETURNING id",
        )
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        let first_session_id = Uuid::new_v4();
        let second_session_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        save(
            &mut tx.context(),
            first_session_id,
            user_id,
            &[1; 32],
            expires_at,
        )
        .await
        .unwrap();
        save(
            &mut tx.context(),
            second_session_id,
            user_id,
            &[2; 32],
            expires_at,
        )
        .await
        .unwrap();
        assert_eq!(
            find_all_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap()
                .len(),
            2
        );

        assert!(
            delete(&mut tx.context(), user_id, first_session_id)
                .await
                .unwrap()
        );
        assert!(
            !delete(&mut tx.context(), user_id, first_session_id)
                .await
                .unwrap()
        );
        let remaining = find_all_by_user_account_id(&mut tx.context(), user_id)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, second_session_id);
        tx.rollback().await.unwrap();
    }
}
