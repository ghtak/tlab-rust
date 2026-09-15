use crate::auth::entity;

pub trait UserService {
    async fn create_user(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> tlab::Result<entity::UserAccount>;
}
