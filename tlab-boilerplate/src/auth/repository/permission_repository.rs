use tlab::{paging::Paging, sqlxdb};

use crate::{
    app_container::{AppDBCtx, AppQueryBuilder},
    auth::entity,
};

pub async fn save(
    context: &mut AppDBCtx<'_>,
    permission: &entity::Permission,
) -> tlab::Result<entity::Permission> {
    let row: (i64, String, Option<String>) = if permission.id == -1 {
        sqlx::query_as(
            "INSERT INTO tlab_permission (code, description) VALUES ($1, $2) RETURNING id, code, description",
        )
        .bind(&permission.code)
        .bind(&permission.description)
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
    } else {
        sqlx::query_as(
            "UPDATE tlab_permission SET code = $1, description = $2 WHERE id = $3 RETURNING id, code, description",
        )
        .bind(&permission.code)
        .bind(&permission.description)
        .bind(permission.id)
        .fetch_optional(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?
        .ok_or(tlab::Error::NotFound("permission".into()))?
    };

    Ok(entity::Permission {
        id: row.0,
        code: row.1,
        description: row.2,
    })
}

pub async fn delete_by_id(context: &mut AppDBCtx<'_>, id: i64) -> tlab::Result<bool> {
    let result = sqlx::query("DELETE FROM tlab_permission WHERE id = $1")
        .bind(id)
        .execute(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;
    Ok(result.rows_affected() > 0)
}

pub struct PermissionSearchCriteria<'a> {
    pub code: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn search(
    context: &mut AppDBCtx<'_>,
    criteria: &PermissionSearchCriteria<'_>,
) -> tlab::Result<Paging<entity::Permission>> {
    let pattern = criteria.code.map(|code| format!("%{code}%"));
    let mut count_builder = AppQueryBuilder::new("SELECT COUNT(*) FROM tlab_permission");
    if let Some(pattern) = pattern.as_deref() {
        count_builder.push(" WHERE code ILIKE ");
        count_builder.push_bind(pattern);
    }
    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(context.backend())
        .await
        .map_err(sqlxdb::postgres::map_error)?;

    let mut builder = AppQueryBuilder::new("SELECT id, code, description FROM tlab_permission");
    if let Some(pattern) = pattern.as_deref() {
        builder.push(" WHERE code ILIKE ");
        builder.push_bind(pattern);
    }
    builder.push(" ORDER BY code LIMIT ");
    builder.push_bind(criteria.limit);
    builder.push(" OFFSET ");
    builder.push_bind(criteria.offset);
    let rows: Vec<(i64, String, Option<String>)> = builder
        .build_query_as()
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

pub async fn find_all_by_ids_for_key_share(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;

    #[tokio::test]
    async fn saves_new_and_existing_permission() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();
        let code = format!("test:permission:{}", uuid::Uuid::new_v4());
        let mut permission = save(
            &mut tx.context(),
            &entity::Permission::new(code.clone(), None),
        )
        .await
        .unwrap();
        assert!(permission.id > 0);

        permission.description = Some("Updated".into());
        let saved = save(&mut tx.context(), &permission).await.unwrap();
        assert_eq!(saved.id, permission.id);
        assert_eq!(saved.description.as_deref(), Some("Updated"));

        let found = search(
            &mut tx.context(),
            &PermissionSearchCriteria {
                code: Some(&code),
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
        assert_eq!(found.total, 1);
        assert_eq!(found.items[0].id, saved.id);

        let all = search(
            &mut tx.context(),
            &PermissionSearchCriteria {
                code: None,
                limit: 1,
                offset: 0,
            },
        )
        .await
        .unwrap();
        assert!(all.total >= 1);
        assert_eq!(all.items.len(), 1);

        permission.id = i64::MAX;
        assert!(matches!(
            save(&mut tx.context(), &permission).await,
            Err(tlab::Error::NotFound(_))
        ));

        tx.rollback().await.unwrap();
    }
}
