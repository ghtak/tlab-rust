use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
};

use crate::{
    api_response::{ApiResponse, ApiResult},
    app_container::AppContainer,
    auth::{access_claims::AccessClaims, permission, repository::permission_repository},
};

pub(super) fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route(
            "/api/v1/auth/permissions",
            get(list_permissions).post(create_permission),
        )
        .route(
            "/api/v1/auth/permissions/{id}",
            axum::routing::delete(delete_permission),
        )
}

async fn delete_permission(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
) -> ApiResult<()> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to delete permission")
    })?;
    let deleted = permission_repository::delete(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to delete permission");
            ApiResponse::internal_error("failed to delete permission")
        })?;

    if !deleted {
        return Err(ApiResponse::not_found("permission not found"));
    }
    Ok(ApiResponse::ok())
}

#[derive(serde::Serialize)]
struct PermissionResponse {
    id: i64,
    code: String,
    description: Option<String>,
}

impl From<crate::auth::entity::Permission> for PermissionResponse {
    fn from(permission: crate::auth::entity::Permission) -> Self {
        Self {
            id: permission.id,
            code: permission.code,
            description: permission.description,
        }
    }
}

#[derive(serde::Deserialize)]
struct CreatePermissionRequest {
    code: String,
    description: Option<String>,
}

async fn create_permission(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    axum::Json(request): axum::Json<CreatePermissionRequest>,
) -> ApiResult<PermissionResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let code = request.code.trim();
    if code.is_empty() || code.chars().count() > 100 {
        return Err(ApiResponse::bad_request("invalid permission code"));
    }
    let description = request
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to create permission")
    })?;
    let created = permission_repository::insert(&mut conn.context(), code, description)
        .await
        .map_err(|error| match error {
            tlab::Error::Conflict(_) => ApiResponse::conflict("permission already exists"),
            error => {
                tracing::error!(?error, "Failed to create permission");
                ApiResponse::internal_error("failed to create permission")
            }
        })?;

    Ok(ApiResponse::created(created.into()))
}

#[derive(serde::Deserialize)]
struct ListPermissionsQuery {
    code: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(serde::Serialize)]
struct PermissionListResponse {
    items: Vec<PermissionResponse>,
    total: i64,
    page: u32,
    page_size: u32,
}

async fn list_permissions(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Query(query): Query<ListPermissionsQuery>,
) -> ApiResult<PermissionListResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err(ApiResponse::bad_request("invalid pagination"));
    }

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to list permissions")
    })?;
    let (permissions, total) = permission_repository::find_page(
        &mut conn.context(),
        query.code.as_deref(),
        i64::from(page_size),
        i64::from(page - 1) * i64::from(page_size),
    )
    .await
    .map_err(|error| {
        tracing::error!(?error, "Failed to list permissions");
        ApiResponse::internal_error("failed to list permissions")
    })?;

    Ok(ApiResponse::data(PermissionListResponse {
        items: permissions
            .into_iter()
            .map(PermissionResponse::from)
            .collect(),
        total,
        page,
        page_size,
    }))
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
    async fn deletes_permission_and_its_role_links() {
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
        let access_token = body["data"]["access_token"].as_str().unwrap();
        let mut conn = container.database.conn().await.unwrap();
        let code = format!("test:delete:{}", uuid::Uuid::new_v4());
        let id: i64 =
            sqlx::query_scalar("INSERT INTO tlab_permission (code) VALUES ($1) RETURNING id")
                .bind(code)
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        sqlx::query(
            "INSERT INTO tlab_role_permission (role_id, permission_id) SELECT id, $1 FROM tlab_role WHERE code = 'admin'",
        )
        .bind(id)
        .execute(conn.context().backend())
        .await
        .unwrap();
        drop(conn);

        let path = format!("/api/v1/auth/permissions/{id}");
        let response = app
            .clone()
            .oneshot(Request::delete(&path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = app
            .clone()
            .oneshot(
                Request::delete(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let mut conn = container.database.conn().await.unwrap();
        let links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tlab_role_permission WHERE permission_id = $1",
        )
        .bind(id)
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        assert_eq!(links, 0);
        drop(conn);

        let response = app
            .oneshot(
                Request::delete(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn creates_permission_and_rejects_invalid_or_duplicate_code() {
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
        let access_token = body["data"]["access_token"].as_str().unwrap();
        let code = format!("test:create:{}", uuid::Uuid::new_v4());

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/permissions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
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
                Request::post("/api/v1/auth/permissions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::from(serde_json::json!({ "code": code }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/permissions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::from(serde_json::json!({ "code": "   " }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/permissions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "code": "test:missing-auth" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let token = container
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
            .oneshot(
                Request::post("/api/v1/auth/permissions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", token.access.token),
                    )
                    .body(Body::from(
                        serde_json::json!({ "code": "test:forbidden" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let mut conn = container.database.conn().await.unwrap();
        sqlx::query("DELETE FROM tlab_permission WHERE id = $1")
            .bind(id)
            .execute(conn.context().backend())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn lists_permissions_for_admin_and_rejects_other_requests() {
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
        assert_eq!(login.status(), StatusCode::OK);
        let login_body: serde_json::Value =
            serde_json::from_slice(&to_bytes(login.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let access_token = login_body["data"]["access_token"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/permissions?code=manage")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["page"], 1);
        assert_eq!(body["data"]["page_size"], 20);
        let permissions = body["data"]["items"].as_array().unwrap();
        assert_eq!(body["data"]["total"], permissions.len());
        let codes: Vec<_> = permissions
            .iter()
            .map(|item| item["code"].as_str().unwrap())
            .collect();
        assert!(codes.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(codes.contains(&"user:manage"));
        assert!(permissions.iter().all(|item| item["id"].is_i64()));
        assert!(
            permissions
                .iter()
                .all(|item| item.get("description").is_some())
        );

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/permissions?code=manage&page=2&page_size=1")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["page"], 2);
        assert_eq!(body["data"]["page_size"], 1);
        assert_eq!(body["data"]["total"], permissions.len());
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"]["items"][0]["code"], codes[1]);

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/permissions?code=USER&page_size=1")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
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
        assert_eq!(body["data"]["items"][0]["code"], "user:manage");

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/permissions?page=0")
                    .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/permissions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let token = container
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
            .oneshot(
                Request::get("/api/v1/auth/permissions")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", token.access.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
