use tlab::{paging::Paging, sqlxdb};

use crate::{app_container::AppDBCtx, auth::entity};

pub async fn insert(
    context: &mut AppDBCtx<'_>,
    code: &str,
    description: Option<&str>,
) -> tlab::Result<entity::Permission> {
    let (id, code, description): (i64, String, Option<String>) = sqlx::query_as(
        "INSERT INTO tlab_permission (code, description) VALUES ($1, $2) RETURNING id, code, description",
    )
    .bind(code)
    .bind(description)
    .fetch_one(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    Ok(entity::Permission {
        id,
        code,
        description,
    })
}

pub async fn delete(context: &mut AppDBCtx<'_>, id: i64) -> tlab::Result<bool> {
    let result = sqlx::query("DELETE FROM tlab_permission WHERE id = $1")
        .bind(id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(result.rows_affected() > 0)
}

pub async fn find(
    context: &mut AppDBCtx<'_>,
    code: Option<&str>,
    limit: i64,
    offset: i64,
) -> tlab::Result<Paging<entity::Permission>> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tlab_permission WHERE ($1::TEXT IS NULL OR code ILIKE '%' || $1 || '%')",
    )
        .bind(code)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        r#"SELECT id, code, description FROM tlab_permission
        WHERE ($1::TEXT IS NULL OR code ILIKE '%' || $1 || '%')
        ORDER BY code LIMIT $2 OFFSET $3"#,
    )
    .bind(code)
    .bind(limit)
    .bind(offset)
    .fetch_all(context.backend())
    .await
    .map_err(sqlxdb::postgres::map_error)?;

    let permissions = rows
        .into_iter()
        .map(|(id, code, description)| entity::Permission {
            id,
            code,
            description,
        })
        .collect();
    Ok(Paging {
        items: permissions,
        total,
    })
}

pub async fn find_by_ids_for_update(
    context: &mut AppDBCtx<'_>,
    ids: &[i64],
) -> tlab::Result<Vec<entity::Permission>> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, code, description FROM tlab_permission WHERE id = ANY($1) FOR KEY SHARE",
    )
    .bind(ids)
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
