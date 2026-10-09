use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    routing::get,
};

use crate::{
    api_response::{ApiResponse, ApiResult},
    app_container::AppContainer,
    auth::{
        access_claims::AccessClaims,
        permission,
        repository::role_repository::{self, DeleteRoleResult},
        usecase::{SetRolePermissionsCommand, SetRolePermissionsUsecase},
    },
};

pub(super) fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth/roles", get(list_roles).post(create_role))
        .route(
            "/api/v1/auth/roles/{id}",
            axum::routing::delete(delete_role),
        )
        .route(
            "/api/v1/auth/roles/{id}/permissions",
            get(list_role_permissions).patch(set_role_permissions),
        )
}

#[derive(serde::Serialize)]
struct RolePermissionsResponse {
    role: RoleResponse,
    permission_ids: Vec<i64>,
    linked_permissions: Vec<LinkedPermissionResponse>,
}

#[derive(serde::Serialize)]
struct LinkedPermissionResponse {
    id: i64,
    code: String,
    description: Option<String>,
}

async fn list_role_permissions(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
) -> ApiResult<RolePermissionsResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to load role permissions")
    })?;
    let role = role_repository::find_by_id(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to load role");
            ApiResponse::internal_error("failed to load role permissions")
        })?
        .ok_or_else(|| ApiResponse::not_found("role not found"))?;
    let linked_permissions = role_repository::find_permissions(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to load role permissions");
            ApiResponse::internal_error("failed to load role permissions")
        })?;
    let mut role_response: RoleResponse = role.into();
    role_response.permission_count = linked_permissions.len() as i64;
    Ok(ApiResponse::data(RolePermissionsResponse {
        role: role_response,
        permission_ids: linked_permissions
            .iter()
            .map(|permission| permission.id)
            .collect(),
        linked_permissions: linked_permissions
            .into_iter()
            .map(|permission| LinkedPermissionResponse {
                id: permission.id,
                code: permission.code,
                description: permission.description,
            })
            .collect(),
    }))
}

#[derive(serde::Deserialize)]
struct SetRolePermissionsRequest {
    add_ids: Vec<i64>,
    remove_ids: Vec<i64>,
}

async fn set_role_permissions(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
    axum::Json(request): axum::Json<SetRolePermissionsRequest>,
) -> ApiResult<()> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let ids: Vec<_> = request
        .add_ids
        .iter()
        .chain(&request.remove_ids)
        .copied()
        .collect();
    if ids.iter().any(|id| *id <= 0)
        || ids.iter().copied().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err(ApiResponse::bad_request("invalid permission ids"));
    }

    SetRolePermissionsUsecase::new(container.database.clone())
        .execute(&SetRolePermissionsCommand {
            role_id: id,
            add_ids: request.add_ids,
            remove_ids: request.remove_ids,
        })
        .await
        .map_err(|error| match error {
            tlab::Error::NotFound(ref resource) if resource == "role" => {
                ApiResponse::not_found("role not found")
            }
            tlab::Error::NotFound(ref resource) if resource == "permission" => {
                ApiResponse::bad_request("permission not found")
            }
            tlab::Error::InvalidOperation(message) => ApiResponse::bad_request(message),
            error => {
                tracing::error!(?error, "Failed to set role permissions");
                ApiResponse::internal_error("failed to set role permissions")
            }
        })?;
    Ok(ApiResponse::ok())
}

#[derive(serde::Serialize)]
struct RoleResponse {
    id: i64,
    code: String,
    description: Option<String>,
    permission_count: i64,
}

impl From<crate::auth::entity::Role> for RoleResponse {
    fn from(role: crate::auth::entity::Role) -> Self {
        Self {
            id: role.id,
            code: role.code,
            description: role.description,
            permission_count: 0,
        }
    }
}

#[derive(serde::Deserialize)]
struct CreateRoleRequest {
    code: String,
    description: Option<String>,
}

async fn create_role(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    axum::Json(request): axum::Json<CreateRoleRequest>,
) -> ApiResult<RoleResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let code = request.code.trim();
    if code.is_empty() || code.chars().count() > 50 {
        return Err(ApiResponse::bad_request("invalid role code"));
    }
    let description = request
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to create role")
    })?;
    let created = role_repository::insert(&mut conn.context(), code, description)
        .await
        .map_err(|error| match error {
            tlab::Error::Conflict(_) => ApiResponse::conflict("role already exists"),
            error => {
                tracing::error!(?error, "Failed to create role");
                ApiResponse::internal_error("failed to create role")
            }
        })?;
    Ok(ApiResponse::created(created.into()))
}

#[derive(serde::Deserialize)]
struct ListRolesQuery {
    code: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(serde::Serialize)]
struct RoleListResponse {
    items: Vec<RoleResponse>,
    total: i64,
    page: u32,
    page_size: u32,
}

async fn list_roles(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Query(query): Query<ListRolesQuery>,
) -> ApiResult<RoleListResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err(ApiResponse::bad_request("invalid pagination"));
    }

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to list roles")
    })?;
    let paging = role_repository::find(
        &mut conn.context(),
        query.code.as_deref(),
        i64::from(page_size),
        i64::from(page - 1) * i64::from(page_size),
    )
    .await
    .map_err(|error| {
        tracing::error!(?error, "Failed to list roles");
        ApiResponse::internal_error("failed to list roles")
    })?;

    Ok(ApiResponse::data(RoleListResponse {
        items: paging
            .items
            .into_iter()
            .map(|item| RoleResponse {
                permission_count: item.permission_count,
                ..item.role.into()
            })
            .collect(),
        total: paging.total,
        page,
        page_size,
    }))
}

async fn delete_role(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
) -> ApiResult<()> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to delete role")
    })?;
    match role_repository::delete(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to delete role");
            ApiResponse::internal_error("failed to delete role")
        })? {
        DeleteRoleResult::Deleted => Ok(ApiResponse::ok()),
        DeleteRoleResult::Protected => Err(ApiResponse::conflict("role cannot be deleted")),
        DeleteRoleResult::NotFound => Err(ApiResponse::not_found("role not found")),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    use crate::test_app;

    #[tokio::test]
    async fn manages_role_permissions_atomically_and_protects_admin() {
        let (app, container) = test_app::setup().await;
        let login = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "email": "admin@localhost", "password": "passwd" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(login.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let token = body["data"]["access_token"].as_str().unwrap();

        let mut conn = container.database.conn().await.unwrap();
        let role_id: i64 =
            sqlx::query_scalar("INSERT INTO tlab_role (code) VALUES ($1) RETURNING id")
                .bind(format!("test-binding-{}", uuid::Uuid::new_v4()))
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let admin_id: i64 = sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
            .fetch_one(conn.context().backend())
            .await
            .unwrap();
        let user_manage_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_permission WHERE code = 'user:manage'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let file_manage_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_permission WHERE code = 'file:manage'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        drop(conn);

        let patch = |id: i64, add_ids: Vec<i64>, remove_ids: Vec<i64>| {
            Request::patch(format!("/api/v1/auth/roles/{id}/permissions"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "add_ids": add_ids, "remove_ids": remove_ids }).to_string(),
                ))
                .unwrap()
        };
        let get = |id: i64| {
            Request::get(format!("/api/v1/auth/roles/{id}/permissions"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };

        let response = app
            .clone()
            .oneshot(patch(role_id, vec![file_manage_id, user_manage_id], vec![]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app.clone().oneshot(get(role_id)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["permission_ids"].as_array().unwrap().len(), 2);
        assert_eq!(body["data"]["role"]["permission_count"], 2);

        let response = app
            .clone()
            .oneshot(patch(role_id, vec![], vec![file_manage_id]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .clone()
            .oneshot(patch(role_id, vec![i64::MAX], vec![user_manage_id]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app.clone().oneshot(get(role_id)).await.unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            body["data"]["permission_ids"],
            serde_json::json!([user_manage_id])
        );

        let response = app
            .clone()
            .oneshot(patch(admin_id, vec![], vec![user_manage_id]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["error"], "admin must retain user:manage");
        let response = app
            .clone()
            .oneshot(patch(role_id, vec![user_manage_id], vec![user_manage_id]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app
            .clone()
            .oneshot(
                Request::patch(format!("/api/v1/auth/roles/{role_id}/permissions"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"add_ids":[],"remove_ids":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let mut conn = container.database.conn().await.unwrap();
        sqlx::query("DELETE FROM tlab_role WHERE id = $1")
            .bind(role_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn lists_creates_and_deletes_roles_with_guards() {
        let (app, container) = test_app::setup().await;
        let login = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "email": "admin@localhost", "password": "passwd" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(login.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let token = body["data"]["access_token"].as_str().unwrap();
        let code = format!("test-role-{}", uuid::Uuid::new_v4());

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/roles")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(
                        serde_json::json!({ "code": format!("  {code}  "), "description": "  " })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["code"], code);
        assert!(body["data"]["description"].is_null());
        let id = body["data"]["id"].as_i64().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/roles")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(serde_json::json!({ "code": code }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/roles")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::from(serde_json::json!({ "code": "   " }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/v1/auth/roles?code={code}&page_size=1"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["items"][0]["id"], id);

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/roles?page=0")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let without_permission = container
            .jwt_codec
            .issue_pair(
                "1",
                Some(serde_json::json!({
                    "role_ids": [],
                    "session_id": uuid::Uuid::new_v4(),
                })),
            )
            .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/roles")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", without_permission.access.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let path = format!("/api/v1/auth/roles/{id}");
        let response = app
            .clone()
            .oneshot(Request::delete(&path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let mut conn = container.database.conn().await.unwrap();
        let user_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_user_account WHERE email = 'admin@localhost'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        sqlx::query("INSERT INTO tlab_user_role (user_account_id, role_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(id)
            .execute(conn.context().backend())
            .await
            .unwrap();
        drop(conn);

        let response = app
            .clone()
            .oneshot(
                Request::delete(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let mut conn = container.database.conn().await.unwrap();
        sqlx::query("DELETE FROM tlab_user_role WHERE user_account_id = $1 AND role_id = $2")
            .bind(user_id)
            .bind(id)
            .execute(conn.context().backend())
            .await
            .unwrap();
        let admin_id: i64 = sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
            .fetch_one(conn.context().backend())
            .await
            .unwrap();
        drop(conn);

        let response = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/v1/auth/roles/{admin_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let response = app
            .clone()
            .oneshot(
                Request::delete(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::delete(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
