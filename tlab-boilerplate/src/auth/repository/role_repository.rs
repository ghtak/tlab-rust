use tlab::{paging::Paging, sqlxdb};

use crate::{
    app_container::{AppDBCtx, AppQueryBuilder},
    auth::entity,
};

pub struct RoleSearchCriteria<'a> {
    pub code: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn save(context: &mut AppDBCtx<'_>, role: &entity::Role) -> tlab::Result<entity::Role> {
    let (id, code, description): (i64, String, Option<String>) = if role.id == -1 {
        sqlx::query_as(
            "INSERT INTO tlab_role (code, description) VALUES ($1, $2) RETURNING id, code, description",
        )
        .bind(&role.code)
        .bind(&role.description)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
    } else {
        sqlx::query_as(
            "UPDATE tlab_role SET code = $1, description = $2 WHERE id = $3 RETURNING id, code, description",
        )
        .bind(&role.code)
        .bind(&role.description)
        .bind(role.id)
        .fetch_optional(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
        .ok_or(tlab::Error::NotFound("role".into()))?
    };

    Ok(entity::Role {
        id,
        code,
        description,
    })
}

pub async fn search(
    context: &mut AppDBCtx<'_>,
    criteria: &RoleSearchCriteria<'_>,
) -> tlab::Result<Paging<entity::RoleWithPermissionCount>> {
    let pattern = criteria.code.map(|code| format!("%{code}%"));
    let mut count_builder = AppQueryBuilder::new("SELECT COUNT(*) FROM tlab_role");
    if let Some(pattern) = pattern.as_deref() {
        count_builder.push(" WHERE code ILIKE ");
        count_builder.push_bind(pattern);
    }
    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    let mut builder = AppQueryBuilder::new(
        r#"SELECT role.id, role.code, role.description,
                  (SELECT COUNT(*) FROM tlab_role_permission AS link WHERE link.role_id = role.id)
           FROM tlab_role AS role"#,
    );
    if let Some(pattern) = pattern.as_deref() {
        builder.push(" WHERE role.code ILIKE ");
        builder.push_bind(pattern);
    }
    builder.push(" ORDER BY role.code LIMIT ");
    builder.push_bind(criteria.limit);
    builder.push(" OFFSET ");
    builder.push_bind(criteria.offset);
    let rows: Vec<(i64, String, Option<String>, i64)> = builder
        .build_query_as()
        .fetch_all(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    Ok(Paging {
        items: rows
            .into_iter()
            .map(
                |(id, code, description, permission_count)| entity::RoleWithPermissionCount {
                    role: entity::Role {
                        id,
                        code,
                        description,
                    },
                    permission_count,
                },
            )
            .collect(),
        total,
    })
}

pub async fn find_by_id(context: &mut AppDBCtx<'_>, id: i64) -> tlab::Result<Option<entity::Role>> {
    let row: Option<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, code, description FROM tlab_role WHERE id = $1")
            .bind(id)
            .fetch_optional(context.backend())
            .await
            .map_err(sqlxdb::postgres::map_error)?;
    Ok(row.map(|(id, code, description)| entity::Role {
        id,
        code,
        description,
    }))
}

pub async fn find_by_id_for_update(
    context: &mut AppDBCtx<'_>,
    id: i64,
) -> tlab::Result<Option<entity::Role>> {
    let row: Option<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, code, description FROM tlab_role WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(context.backend())
            .await
            .map_err(sqlxdb::postgres::map_error)?;
    Ok(row.map(|(id, code, description)| entity::Role {
        id,
        code,
        description,
    }))
}

pub async fn find_all_by_ids_for_key_share(
    context: &mut AppDBCtx<'_>,
    ids: &[i64],
) -> tlab::Result<Vec<entity::Role>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, code, description FROM tlab_role WHERE id = ANY($1) FOR KEY SHARE",
    )
    .bind(ids)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(rows
        .into_iter()
        .map(|(id, code, description)| entity::Role {
            id,
            code,
            description,
        })
        .collect())
}

pub async fn find_all_permissions_by_id(
    context: &mut AppDBCtx<'_>,
    role_id: i64,
) -> tlab::Result<Vec<entity::Permission>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT permission.id, permission.code, permission.description \
         FROM tlab_role_permission AS link \
         JOIN tlab_permission AS permission ON permission.id = link.permission_id \
         WHERE link.role_id = $1 ORDER BY permission.code",
    )
    .bind(role_id)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(rows
        .into_iter()
        .map(|(id, code, description)| entity::Permission {
            id,
            code,
            description,
        })
        .collect())
}

pub async fn find_all_permission_codes_by_ids(
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

pub async fn replace_permissions(
    context: &mut AppDBCtx<'_>,
    role_id: i64,
    permission_ids: &[i64],
) -> tlab::Result<()> {
    sqlx::query("DELETE FROM tlab_role_permission WHERE role_id = $1")
        .bind(role_id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    sqlx::query(
        "INSERT INTO tlab_role_permission (role_id, permission_id) \
         SELECT $1, id FROM UNNEST($2::BIGINT[]) AS permission_id(id) ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .bind(permission_ids)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub async fn delete_by_id(context: &mut AppDBCtx<'_>, id: i64) -> tlab::Result<bool> {
    let deleted = sqlx::query(
        r#"DELETE FROM tlab_role AS role
        WHERE role.id = $1 AND role.code <> 'admin'
          AND NOT EXISTS (SELECT 1 FROM tlab_user_role WHERE role_id = role.id)"#,
    )
    .bind(id)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    Ok(deleted.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;

    #[tokio::test]
    async fn finds_permission_codes_for_roles() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let role = save(
            &mut tx.context(),
            &entity::Role::new(format!("test-role-{suffix}"), None),
        )
        .await
        .unwrap();
        let updated = save(
            &mut tx.context(),
            &entity::Role {
                description: Some("updated".into()),
                ..role.clone()
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.description.as_deref(), Some("updated"));
        let code = format!("test:permission:{suffix}");
        let permission_id: i64 =
            sqlx::query_scalar("INSERT INTO tlab_permission (code) VALUES ($1) RETURNING id")
                .bind(&code)
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        sqlx::query("INSERT INTO tlab_role_permission (role_id, permission_id) VALUES ($1, $2)")
            .bind(role.id)
            .bind(permission_id)
            .execute(tx.context().backend())
            .await
            .unwrap();

        assert_eq!(
            find_all_permission_codes_by_ids(&mut tx.context(), &[role.id])
                .await
                .unwrap(),
            [code]
        );
        assert!(
            find_all_permission_codes_by_ids(&mut tx.context(), &[])
                .await
                .unwrap()
                .is_empty()
        );

        tx.rollback().await.unwrap();
    }
}
