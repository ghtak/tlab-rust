use tlab::sqlxdb;

use crate::app_container::AppDBCtx;

pub async fn find_role_ids_by_user_account_id(
    context: &mut AppDBCtx<'_>,
    user_account_id: i64,
) -> tlab::Result<Vec<i64>> {
    sqlx::query_scalar(
        "SELECT role_id FROM tlab_user_role WHERE user_account_id = $1 ORDER BY role_id",
    )
    .bind(user_account_id)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)
}

pub async fn find_permission_codes_by_role_ids(
    context: &mut AppDBCtx<'_>,
    role_ids: &[i64],
) -> tlab::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT DISTINCT permission.code FROM tlab_role_permission AS role_permission \
         JOIN tlab_permission AS permission ON permission.id = role_permission.permission_id \
         WHERE role_permission.role_id = ANY($1) ORDER BY permission.code",
    )
    .bind(role_ids)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migration::AUTH_MIGRATIONS, test_db};

    #[tokio::test]
    #[ignore = "requires tlab-boilerplate Docker PostgreSQL service"]
    async fn loads_assigned_roles_and_their_permissions() {
        let database = test_db::connect().await;
        let mut tx =
            test_db::isolated_tx(&database, &[AUTH_MIGRATIONS[0].1, AUTH_MIGRATIONS[3].1]).await;

        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Alice', 'alice@example.com') RETURNING id",
        )
        .fetch_one(tx.context().backend())
        .await
        .unwrap();
        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        let sales_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'sales'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        sqlx::query("INSERT INTO tlab_user_role (user_account_id, role_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(admin_role_id)
            .execute(tx.context().backend())
            .await
            .unwrap();
        sqlx::query("INSERT INTO tlab_user_role (user_account_id, role_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(sales_role_id)
            .execute(tx.context().backend())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tlab_role_permission (role_id, permission_id) \
             SELECT $1, id FROM tlab_permission WHERE code = 'file:manage'",
        )
        .bind(sales_role_id)
        .execute(tx.context().backend())
        .await
        .unwrap();

        assert_eq!(
            find_role_ids_by_user_account_id(&mut tx.context(), user_id)
                .await
                .unwrap(),
            [admin_role_id, sales_role_id]
        );
        assert!(
            find_role_ids_by_user_account_id(&mut tx.context(), user_id + 1)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            find_permission_codes_by_role_ids(&mut tx.context(), &[admin_role_id, sales_role_id])
                .await
                .unwrap(),
            ["file:manage", "user:manage"]
        );
        assert_eq!(
            find_permission_codes_by_role_ids(&mut tx.context(), &[sales_role_id])
                .await
                .unwrap(),
            ["file:manage"]
        );
        assert!(
            find_permission_codes_by_role_ids(&mut tx.context(), &[])
                .await
                .unwrap()
                .is_empty()
        );

        tx.rollback().await.unwrap();
    }
}
