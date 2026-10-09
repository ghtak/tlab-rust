use tlab::sqlxdb;

use crate::{app_container::AppDBCtx, auth::entity};

pub async fn find_page(
    context: &mut AppDBCtx<'_>,
    code: Option<&str>,
    limit: i64,
    offset: i64,
) -> tlab::Result<(Vec<entity::Permission>, i64)> {
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
    Ok((permissions, total))
}
