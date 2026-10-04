use chrono::{DateTime, Utc};
use tlab::sqlxdb;

use crate::{app_container::AppDBCtx, auth::entity};

pub async fn save(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
    token_hash: &[u8; 32],
    expires_at: DateTime<Utc>,
) -> tlab::Result<()> {
    sqlx::query(
        r#"INSERT INTO tlab_refresh_token (user_account_id, token_hash, expires_at)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_account_id) DO UPDATE
           SET token_hash = EXCLUDED.token_hash,
               created_at = CURRENT_TIMESTAMP,
               expires_at = EXCLUDED.expires_at"#,
    )
    .bind(user_account_id)
    .bind(token_hash.as_slice())
    .bind(expires_at)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub async fn find_by_user_account_id(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Option<entity::RefreshToken>> {
    let row = sqlx::query_as::<_, (i64, Vec<u8>, DateTime<Utc>, DateTime<Utc>)>(
        r#"SELECT user_account_id, token_hash, created_at, expires_at
           FROM tlab_refresh_token WHERE user_account_id = $1"#,
    )
    .bind(user_account_id)
    .fetch_optional(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(row.map(
        |(user_account_id, token_hash, created_at, expires_at)| entity::RefreshToken {
            user_account_id,
            token_hash,
            created_at,
            expires_at,
        },
    ))
}

pub async fn delete(context: &mut AppDBCtx<'_>, user_account_id: i64) -> tlab::Result<bool> {
    let result = sqlx::query(r#"DELETE FROM tlab_refresh_token WHERE user_account_id = $1"#)
        .bind(user_account_id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(result.rows_affected() != 0)
}
