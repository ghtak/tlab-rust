mod rbac_service;
mod token_service;

pub use rbac_service::{PermissionCheckStrategy, RbacService};
pub use token_service::TokenService;

// pub trait UserService {
//     // async fn create_user(
//     //     &self,
//     //     name: &str,
//     //     email: &str,
//     //     password: &str,
//     // ) -> tlab::Result<entity::UserAccount>;
// }
