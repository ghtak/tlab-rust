use tlab::{paging::Paging, sqlxdb};

use crate::{app_container::AppDBCtx, auth::entity};

pub struct RoleWithPermissionCount {
    pub role: entity::Role,
    pub permission_count: i64,
}

pub async fn insert(
    context: &mut AppDBCtx<'_>,
    code: &str,
    description: Option<&str>,
) -> tlab::Result<entity::Role> {
    let (id, code, description): (i64, String, Option<String>) = sqlx::query_as(
        "INSERT INTO tlab_role (code, description) VALUES ($1, $2) RETURNING id, code, description",
    )
    .bind(code)
    .bind(description)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(entity::Role {
        id,
        code,
        description,
    })
}

pub async fn find(
    context: &mut AppDBCtx<'_>,
    code: Option<&str>,
    limit: i64,
    offset: i64,
) -> tlab::Result<Paging<RoleWithPermissionCount>> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tlab_role WHERE ($1::TEXT IS NULL OR code ILIKE '%' || $1 || '%')",
    )
    .bind(code)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    let rows: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
        r#"SELECT role.id, role.code, role.description, COUNT(link.permission_id)
        FROM tlab_role AS role
        LEFT JOIN tlab_role_permission AS link ON link.role_id = role.id
        WHERE ($1::TEXT IS NULL OR role.code ILIKE '%' || $1 || '%')
        GROUP BY role.id ORDER BY role.code LIMIT $2 OFFSET $3"#,
    )
    .bind(code)
    .bind(limit)
    .bind(offset)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(Paging {
        items: rows
            .into_iter()
            .map(
                |(id, code, description, permission_count)| RoleWithPermissionCount {
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

pub async fn find_permissions(
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

pub async fn add_permissions(
    context: &mut AppDBCtx<'_>,
    role_id: i64,
    permission_ids: &[i64],
) -> tlab::Result<()> {
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

pub async fn remove_permissions(
    context: &mut AppDBCtx<'_>,
    role_id: i64,
    permission_ids: &[i64],
) -> tlab::Result<()> {
    sqlx::query("DELETE FROM tlab_role_permission WHERE role_id = $1 AND permission_id = ANY($2)")
        .bind(role_id)
        .bind(permission_ids)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(())
}

pub enum DeleteRoleResult {
    Deleted,
    Protected,
    NotFound,
}

pub async fn delete(context: &mut AppDBCtx<'_>, id: i64) -> tlab::Result<DeleteRoleResult> {
    let deleted = sqlx::query(
        r#"DELETE FROM tlab_role AS role
        WHERE role.id = $1 AND role.code <> 'admin'
          AND NOT EXISTS (SELECT 1 FROM tlab_user_role WHERE role_id = role.id)"#,
    )
    .bind(id)
    .execute(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    if deleted.rows_affected() > 0 {
        return Ok(DeleteRoleResult::Deleted);
    }

    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tlab_role WHERE id = $1)")
        .bind(id)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(if exists {
        DeleteRoleResult::Protected
    } else {
        DeleteRoleResult::NotFound
    })
}
