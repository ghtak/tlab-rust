use crate::{app_container::AppDbContext, auth::entity};

pub trait UserService: Sync + Send + 'static {
    // Add any dependencies or configuration needed for the service
    async fn create_user(
        &self,
        context: &mut AppDbContext<'_>,
        name: String,
        email: String,
        password: String,
    ) -> tlab::Result<entity::UserAccount>;
}
