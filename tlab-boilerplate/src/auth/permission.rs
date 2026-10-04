use crate::{
    api_response::ApiResponse, app_container::AppContainer, auth::access_claims::AccessClaims,
};

pub const USER_MANAGE: &str = "user:manage";

pub async fn require(
    container: &AppContainer,
    claims: &AccessClaims,
    permission: &str,
) -> Result<(), ApiResponse<()>> {
    let mut conn = container.database.conn().await.map_err(|error| {
        tracing::error!(?error, "Failed to connect to database");
        ApiResponse::internal_error("failed to check permission")
    })?;
    let allowed = container
        .rbac_service
        .has_permission(&mut conn.context(), &claims.app.role_ids, permission)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to check permission");
            ApiResponse::internal_error("failed to check permission")
        })?;

    if !allowed {
        return Err(ApiResponse::forbidden("permission denied"));
    }

    Ok(())
}
