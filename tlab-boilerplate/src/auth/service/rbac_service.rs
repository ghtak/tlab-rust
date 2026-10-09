use crate::{
    app_container::AppDBCtx,
    auth::{
        entity::{Permission, Role, UserAccount},
        repository::role_repository,
    },
};

pub struct RbacService;

impl RbacService {
    pub fn validate_user_role_change(&self, user: &UserAccount, roles: &[Role]) -> tlab::Result<()> {
        if user.email == "admin@localhost" && !roles.iter().any(|role| role.code == "admin") {
            return Err(tlab::Error::InvalidOperation(
                "initial admin must retain admin role".into(),
            ));
        }
        Ok(())
    }

    pub fn validate_role_permission_change(
        &self,
        role: &Role,
        permissions: &[Permission],
    ) -> tlab::Result<()> {
        if role.code == "admin"
            && !permissions
                .iter()
                .any(|permission| permission.code == "user:manage")
        {
            return Err(tlab::Error::InvalidOperation(
                "admin must retain user:manage".into(),
            ));
        }
        Ok(())
    }

    pub async fn has_permission(
        &self,
        context: &mut AppDBCtx<'_>,
        role_ids: &[i64],
        permission_code: &str,
    ) -> tlab::Result<bool> {
        if role_ids.is_empty() {
            return Ok(false);
        }

        let permission_codes =
            role_repository::find_all_permission_codes_by_ids(context, role_ids).await?;
        Ok(permission_codes.iter().any(|code| code == permission_code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;

    #[tokio::test]
    async fn checks_permissions_from_all_assigned_roles() {
        let database = test_db::connect().await;
        let mut tx = database.tx().await.unwrap();

        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        let sales_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'sales'")
                .fetch_one(tx.context().backend())
                .await
                .unwrap();
        let service = RbacService;

        assert!(
            service
                .has_permission(
                    &mut tx.context(),
                    &[sales_role_id, admin_role_id],
                    "user:manage"
                )
                .await
                .unwrap()
        );
        assert!(
            !service
                .has_permission(&mut tx.context(), &[sales_role_id], "user:manage")
                .await
                .unwrap()
        );
        assert!(
            !service
                .has_permission(&mut tx.context(), &[], "user:manage")
                .await
                .unwrap()
        );

        tx.rollback().await.unwrap();
    }
}
