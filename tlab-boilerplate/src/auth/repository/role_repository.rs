use tlab::sqlxdb;

use crate::{app_container::AppDBCtx, auth::entity};

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

pub async fn find_page(
    context: &mut AppDBCtx<'_>,
    code: Option<&str>,
    limit: i64,
    offset: i64,
) -> tlab::Result<(Vec<entity::Role>, i64)> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tlab_role WHERE ($1::TEXT IS NULL OR code ILIKE '%' || $1 || '%')",
    )
    .bind(code)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        r#"SELECT id, code, description FROM tlab_role
        WHERE ($1::TEXT IS NULL OR code ILIKE '%' || $1 || '%')
        ORDER BY code LIMIT $2 OFFSET $3"#,
    )
    .bind(code)
    .bind(limit)
    .bind(offset)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok((
        rows.into_iter()
            .map(|(id, code, description)| entity::Role {
                id,
                code,
                description,
            })
            .collect(),
        total,
    ))
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
