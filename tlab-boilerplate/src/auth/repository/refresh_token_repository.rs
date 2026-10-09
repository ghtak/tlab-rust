#![allow(dead_code)]

use chrono::{DateTime, Utc};
use tlab::sqlxdb;
use uuid::Uuid;

use crate::{
    app_container::{AppDBCtx, AppQueryBuilder},
    auth::entity,
};

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    session_id: Uuid,
    user_account_id: i64,
    token_hash: Vec<u8>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl RefreshTokenRow {
    fn into_entity(self) -> entity::RefreshToken {
        entity::RefreshToken {
            session_id: self.session_id,
            user_account_id: self.user_account_id,
            token_hash: self.token_hash,
            created_at: self.created_at,
            expires_at: self.expires_at,
        }
    }
}

pub struct RefreshTokenCondition<'a> {
    pub user_account_id: i64,
    pub session_id: Uuid,
    pub token_hash: &'a [u8; 32],
    pub status: Option<entity::UserStatus>,
}

pub async fn save(context: &mut AppDBCtx<'_>, token: &entity::RefreshToken) -> tlab::Result<()> {
    sqlx::query(
        r#"INSERT INTO tlab_refresh_token (session_id, user_account_id, token_hash, expires_at)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (session_id) DO UPDATE
           SET token_hash = EXCLUDED.token_hash, expires_at = EXCLUDED.expires_at"#,
    )
    .bind(token.session_id)
    .bind(token.user_account_id)
    .bind(&token.token_hash)
    .bind(token.expires_at)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub async fn find_by_condition(
    context: &mut AppDBCtx<'_>,
    condition: &RefreshTokenCondition<'_>,
) -> tlab::Result<Option<entity::RefreshToken>> {
    find(context, condition, false).await
}

pub async fn find_by_condition_for_update(
    context: &mut AppDBCtx<'_>,
    condition: &RefreshTokenCondition<'_>,
) -> tlab::Result<Option<entity::RefreshToken>> {
    find(context, condition, true).await
}

async fn find(
    context: &mut AppDBCtx<'_>,
    condition: &RefreshTokenCondition<'_>,
    for_update: bool,
) -> tlab::Result<Option<entity::RefreshToken>> {
    let mut builder = AppQueryBuilder::new(
        r#"SELECT token.session_id, token.user_account_id, token.token_hash,
                  token.created_at, token.expires_at
           FROM tlab_refresh_token AS token
           JOIN tlab_user_account AS account ON account.id = token.user_account_id
           WHERE token.user_account_id = "#,
    );
    builder.push_bind(condition.user_account_id);
    builder.push(" AND token.session_id = ");
    builder.push_bind(condition.session_id);
    builder.push(" AND token.token_hash = ");
    builder.push_bind(condition.token_hash.as_slice());
    if let Some(status) = condition.status {
        builder.push(" AND account.status = ");
        builder.push_bind(status.as_str());
    }
    if for_update {
        builder.push(" FOR UPDATE OF token");
    }
    let row: Option<RefreshTokenRow> = builder
        .build_query_as()
        .fetch_optional(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    Ok(row.map(RefreshTokenRow::into_entity))
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

pub async fn delete_by_user_account_id_and_session_id(
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
    use crate::test_db;

    #[tokio::test]
    async fn stores_multiple_sessions_and_deletes_only_one() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Alice', $1) RETURNING id",
        )
        .bind(test_db::unique_email("refresh-token"))
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        let first_session_id = Uuid::new_v4();
        let second_session_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        save(
            &mut tx.context(),
            &entity::RefreshToken::new(first_session_id, user_id, vec![1; 32], expires_at),
        )
        .await
        .unwrap();
        save(
            &mut tx.context(),
            &entity::RefreshToken::new(second_session_id, user_id, vec![2; 32], expires_at),
        )
        .await
        .unwrap();
        let condition = RefreshTokenCondition {
            user_account_id: user_id,
            session_id: first_session_id,
            token_hash: &[1; 32],
            status: Some(entity::UserStatus::Active),
        };
        let session = find_by_condition(&mut tx.context(), &condition)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(session.user_account_id, user_id);
        assert!(find_by_condition(
            &mut tx.context(),
            &RefreshTokenCondition {
                token_hash: &[3; 32],
                ..condition
            }
        )
        .await
        .unwrap()
        .is_none());
        sqlx::query("UPDATE tlab_user_account SET status = 'suspended' WHERE id = $1")
            .bind(user_id)
            .execute(tx.context().backend())
            .await
            .unwrap();
        assert!(find_by_condition(&mut tx.context(), &condition)
            .await
            .unwrap()
            .is_none());
        let session = find_by_condition_for_update(
            &mut tx.context(),
            &RefreshTokenCondition {
                status: None,
                ..condition
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(session.session_id, first_session_id);
        assert_eq!(
            find_all_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap()
                .len(),
            2
        );

        assert!(delete_by_user_account_id_and_session_id(&mut tx.context(), user_id, first_session_id)
            .await
            .unwrap());
        assert!(!delete_by_user_account_id_and_session_id(&mut tx.context(), user_id, first_session_id)
            .await
            .unwrap());
        let remaining = find_all_by_user_account_id(&mut tx.context(), user_id)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, second_session_id);
        tx.rollback().await.unwrap();
    }
}
