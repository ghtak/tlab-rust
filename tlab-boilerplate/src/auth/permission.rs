use crate::{
    api_response::ApiResponse, app_container::AppContainer, auth::access_claims::AccessClaims,
};

pub use crate::auth::service::PermissionCheckStrategy;

pub const ACCESS_MANAGE: &str = "access:manage";

pub async fn require(
    container: &AppContainer,
    claims: &AccessClaims,
    permission: &str,
    strategy: PermissionCheckStrategy,
) -> Result<(), ApiResponse<()>> {
    let allowed = container
        .rbac_service
        .has_permission(&claims.app.role_ids, permission, strategy)
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
