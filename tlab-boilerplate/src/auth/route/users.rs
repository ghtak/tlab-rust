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
        entity::{Role, UserStatus},
        permission,
        repository::user_repository,
        usecase::{SetUserRolesCommand, SetUserRolesUsecase},
    },
};

pub(super) fn router() -> axum::Router<Arc<AppContainer>> {
    axum::Router::new()
        .route("/api/v1/auth/users", get(list_users))
        .route(
            "/api/v1/auth/users/{id}/roles",
            get(list_user_roles).patch(set_user_roles),
        )
}

#[derive(serde::Serialize)]
struct UserRoleSubjectResponse {
    id: i64,
    name: String,
    email: String,
    status: String,
}

#[derive(serde::Serialize)]
struct UserRolesResponse {
    user: UserRoleSubjectResponse,
    roles: Vec<Role>,
}

async fn list_user_roles(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
) -> ApiResult<UserRolesResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to load user roles")
    })?;
    let user = user_repository::find_user_account(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to load user");
            ApiResponse::internal_error("failed to load user roles")
        })?
        .ok_or_else(|| ApiResponse::not_found("user not found"))?;
    let roles = user_repository::find_roles_by_user_account_id(&mut conn.context(), id)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to load user roles");
            ApiResponse::internal_error("failed to load user roles")
        })?;
    Ok(ApiResponse::data(UserRolesResponse {
        user: UserRoleSubjectResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            status: user.status.as_str().to_owned(),
        },
        roles,
    }))
}

#[derive(serde::Deserialize)]
struct SetUserRolesRequest {
    add_ids: Vec<i64>,
    remove_ids: Vec<i64>,
}

async fn set_user_roles(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Path(id): Path<i64>,
    axum::Json(request): axum::Json<SetUserRolesRequest>,
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
        return Err(ApiResponse::bad_request("invalid role ids"));
    }

    SetUserRolesUsecase::new(container.database.clone())
        .execute(&SetUserRolesCommand {
            user_account_id: id,
            add_ids: request.add_ids,
            remove_ids: request.remove_ids,
        })
        .await
        .map_err(|error| match error {
            tlab::Error::NotFound(ref resource) if resource == "user" => {
                ApiResponse::not_found("user not found")
            }
            tlab::Error::NotFound(ref resource) if resource == "role" => {
                ApiResponse::bad_request("role not found")
            }
            tlab::Error::InvalidOperation(message) => ApiResponse::bad_request(message),
            error => {
                tracing::error!(?error, "Failed to set user roles");
                ApiResponse::internal_error("failed to set user roles")
            }
        })?;
    Ok(ApiResponse::ok())
}

#[derive(serde::Deserialize)]
struct ListUsersQuery {
    q: Option<String>,
    status: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(serde::Serialize)]
struct UserListItemResponse {
    id: i64,
    name: String,
    email: String,
    status: String,
    roles: Vec<String>,
    providers: Vec<String>,
}

#[derive(serde::Serialize)]
struct UserListResponse {
    items: Vec<UserListItemResponse>,
    total: i64,
    page: u32,
    page_size: u32,
}

async fn list_users(
    State(container): State<Arc<AppContainer>>,
    claims: AccessClaims,
    Query(query): Query<ListUsersQuery>,
) -> ApiResult<UserListResponse> {
    permission::require(&container, &claims, permission::USER_MANAGE).await?;

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err(ApiResponse::bad_request("invalid pagination"));
    }
    let status = query
        .status
        .as_deref()
        .map(str::parse::<UserStatus>)
        .transpose()
        .map_err(|_| ApiResponse::bad_request("invalid status"))?;
    let search = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());

    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to list users")
    })?;
    let paging = user_repository::find_user_accounts(
        &mut conn.context(),
        search,
        status,
        i64::from(page_size),
        i64::from(page - 1) * i64::from(page_size),
    )
    .await
    .map_err(|error| {
        tracing::error!(?error, "Failed to list users");
        ApiResponse::internal_error("failed to list users")
    })?;

    Ok(ApiResponse::data(UserListResponse {
        items: paging
            .items
            .into_iter()
            .map(|item| UserListItemResponse {
                id: item.account.id,
                name: item.account.name,
                email: item.account.email,
                status: item.account.status.as_str().to_owned(),
                roles: item.roles,
                providers: item.providers,
            })
            .collect(),
        total: paging.total,
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
    async fn changes_user_roles_and_protects_initial_admin() {
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
        let email = format!("roles-{}@example.com", uuid::Uuid::new_v4().simple());
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email) VALUES ('Role test', $1) RETURNING id",
        )
        .bind(email)
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        let admin_id: i64 = sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
            .fetch_one(conn.context().backend())
            .await
            .unwrap();
        let sales_id: i64 = sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'sales'")
            .fetch_one(conn.context().backend())
            .await
            .unwrap();

        let path = format!("/api/v1/auth/users/{user_id}/roles");
        let response = app
            .clone()
            .oneshot(
                Request::patch(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "add_ids": [admin_id, sales_id], "remove_ids": [] })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::get(&path)
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
        assert_eq!(body["data"]["user"]["id"], user_id);
        assert_eq!(body["data"]["roles"][0]["code"], "admin");
        assert_eq!(body["data"]["roles"][1]["code"], "sales");

        let response = app
            .clone()
            .oneshot(
                Request::patch(&path)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "add_ids": [], "remove_ids": [sales_id] }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let role_ids: Vec<i64> = sqlx::query_scalar(
            "SELECT role_id FROM tlab_user_role WHERE user_account_id = $1 ORDER BY role_id",
        )
        .bind(user_id)
        .fetch_all(conn.context().backend())
        .await
        .unwrap();
        assert_eq!(role_ids, vec![admin_id]);

        let initial_admin_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_user_account WHERE email = 'admin@localhost'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::patch(format!("/api/v1/auth/users/{initial_admin_id}/roles"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "add_ids": [], "remove_ids": [admin_id] }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["error"], "initial admin must retain admin role");

        sqlx::query("DELETE FROM tlab_user_account WHERE id = $1")
            .bind(user_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn lists_filtered_users_with_roles_and_requires_permission() {
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

        let marker = format!("listprobe{}", uuid::Uuid::new_v4().simple());
        let mut conn = container.database.conn().await.unwrap();
        let active_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email, status) VALUES ($1, $2, 'active') RETURNING id",
        )
        .bind(&marker)
        .bind(format!("{marker}-active@example.com"))
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        let suspended_id: i64 = sqlx::query_scalar(
            "INSERT INTO tlab_user_account (name, email, status) VALUES ($1, $2, 'suspended') RETURNING id",
        )
        .bind(&marker)
        .bind(format!("{marker}-suspended@example.com"))
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tlab_user_role (user_account_id, role_id) SELECT $1, id FROM tlab_role WHERE code IN ('admin', 'sales')",
        )
        .bind(active_id)
        .execute(conn.context().backend())
        .await
        .unwrap();
        for provider in ["managed", "google"] {
            sqlx::query(
                "INSERT INTO tlab_user_identity (user_account_id, provider, provider_subject) VALUES ($1, $2, $3)",
            )
            .bind(active_id)
            .bind(provider)
            .bind(format!("{marker}-{provider}"))
            .execute(conn.context().backend())
            .await
            .unwrap();
        }

        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/v1/auth/users?q={marker}&status=active&page_size=1"
                ))
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
        assert_eq!(body["data"]["page"], 1);
        assert_eq!(body["data"]["items"][0]["id"], active_id);
        assert_eq!(
            body["data"]["items"][0]["roles"],
            serde_json::json!(["admin", "sales"])
        );
        assert_eq!(
            body["data"]["items"][0]["providers"],
            serde_json::json!(["google", "managed"])
        );

        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/v1/auth/users?q={marker}&page=2&page_size=1"))
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
        assert_eq!(body["data"]["total"], 2);
        assert_eq!(body["data"]["items"][0]["id"], active_id);

        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/v1/auth/users?q={marker}-suspended%40example.com"
                ))
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
        assert_eq!(body["data"]["items"][0]["id"], suspended_id);
        assert_eq!(body["data"]["items"][0]["roles"], serde_json::json!([]));
        assert_eq!(body["data"]["items"][0]["providers"], serde_json::json!([]));

        for path in [
            "/api/v1/auth/users?page=0",
            "/api/v1/auth/users?status=inactive",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(path)
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/auth/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

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
            .oneshot(
                Request::get("/api/v1/auth/users")
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

        sqlx::query("DELETE FROM tlab_user_identity WHERE user_account_id = $1")
            .bind(active_id)
            .execute(conn.context().backend())
            .await
            .unwrap();
        sqlx::query("DELETE FROM tlab_user_account WHERE id = ANY($1)")
            .bind(&[active_id, suspended_id][..])
            .execute(conn.context().backend())
            .await
            .unwrap();
    }
}
