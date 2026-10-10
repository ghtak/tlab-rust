use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use crate::{
    app_container::AppDB,
    auth::{
        entity::{Permission, Role, UserAccount},
        repository::role_repository,
    },
};

const CACHE_TTL: Duration = Duration::from_secs(600);
const CACHE_CAPACITY: usize = 1024;

#[derive(Clone, Copy, Default)]
pub enum PermissionCheckStrategy {
    #[default]
    Cached,
    Direct,
}

struct CachedPermissions {
    codes: HashSet<String>,
    loaded_at: Instant,
}

pub struct RbacService {
    database: Arc<AppDB>,
    cache: RwLock<HashMap<Vec<i64>, CachedPermissions>>,
}

impl RbacService {
    pub fn new(database: Arc<AppDB>) -> Self {
        Self {
            database,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn validate_user_role_change(user: &UserAccount, roles: &[Role]) -> tlab::Result<()> {
        if user.email == "admin@localhost" && !roles.iter().any(|role| role.code == "admin") {
            return Err(tlab::Error::InvalidOperation(
                "initial admin must retain admin role".into(),
            ));
        }
        Ok(())
    }

    pub fn validate_role_permission_change(
        role: &Role,
        permissions: &[Permission],
    ) -> tlab::Result<()> {
        if role.code == "admin"
            && !permissions
                .iter()
                .any(|permission| permission.code == "access:manage")
        {
            return Err(tlab::Error::InvalidOperation(
                "admin must retain access:manage".into(),
            ));
        }
        Ok(())
    }

    pub async fn has_permission(
        &self,
        role_ids: &[i64],
        permission_code: &str,
        strategy: PermissionCheckStrategy,
    ) -> tlab::Result<bool> {
        if role_ids.is_empty() {
            return Ok(false);
        }

        let mut key = role_ids.to_vec();
        key.sort_unstable();
        key.dedup();

        if matches!(strategy, PermissionCheckStrategy::Cached) {
            let cache = self.cache.read().await;
            if let Some(entry) = cache
                .get(&key)
                .filter(|entry| entry.loaded_at.elapsed() < CACHE_TTL)
            {
                return Ok(entry.codes.contains(permission_code));
            }
        }

        if matches!(strategy, PermissionCheckStrategy::Direct) {
            let mut conn = self.database.conn().await?;
            let codes =
                role_repository::find_all_permission_codes_by_ids(&mut conn.context(), &key)
                    .await?;
            return Ok(codes.iter().any(|code| code == permission_code));
        }

        let mut cache = self.cache.write().await;
        if let Some(entry) = cache
            .get(&key)
            .filter(|entry| entry.loaded_at.elapsed() < CACHE_TTL)
        {
            return Ok(entry.codes.contains(permission_code));
        }

        let mut conn = self.database.conn().await?;
        let codes =
            role_repository::find_all_permission_codes_by_ids(&mut conn.context(), &key).await?;
        if cache.len() >= CACHE_CAPACITY {
            cache.retain(|_, entry| entry.loaded_at.elapsed() < CACHE_TTL);
            if cache.len() >= CACHE_CAPACITY {
                cache.clear();
            }
        }
        let allowed = codes.iter().any(|code| code == permission_code);
        cache.insert(
            key,
            CachedPermissions {
                codes: codes.into_iter().collect(),
                loaded_at: Instant::now(),
            },
        );
        Ok(allowed)
    }

    pub async fn invalidate_role(&self, role_id: i64) {
        self.cache
            .write()
            .await
            .retain(|role_ids, _| !role_ids.contains(&role_id));
    }

    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_db;

    #[tokio::test]
    async fn checks_permissions_from_all_assigned_roles() {
        let database = test_db::connect().await;
        let mut conn = database.conn().await.unwrap();

        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let sales_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'sales'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let permission_code: String = sqlx::query_scalar(
            "SELECT p.code FROM tlab_role_permission AS rp \
             JOIN tlab_permission AS p ON p.id = rp.permission_id \
             WHERE rp.role_id = $1 LIMIT 1",
        )
        .bind(admin_role_id)
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        let service = RbacService::new(database.clone());

        assert!(
            service
                .has_permission(
                    &[sales_role_id, admin_role_id],
                    &permission_code,
                    PermissionCheckStrategy::Direct,
                )
                .await
                .unwrap()
        );
        assert!(
            !service
                .has_permission(
                    &[sales_role_id],
                    &permission_code,
                    PermissionCheckStrategy::Direct,
                )
                .await
                .unwrap()
        );
        assert!(
            !service
                .has_permission(&[], &permission_code, PermissionCheckStrategy::Direct)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn cached_check_reuses_result_and_direct_check_reads_database() {
        let database = test_db::connect().await;
        let mut conn = database.conn().await.unwrap();
        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM tlab_role WHERE code = 'admin'")
                .fetch_one(conn.context().backend())
                .await
                .unwrap();
        let permission_code: String = sqlx::query_scalar(
            "SELECT p.code FROM tlab_role_permission AS rp \
             JOIN tlab_permission AS p ON p.id = rp.permission_id \
             WHERE rp.role_id = $1 LIMIT 1",
        )
        .bind(admin_role_id)
        .fetch_one(conn.context().backend())
        .await
        .unwrap();
        drop(conn);
        let service = RbacService::new(database);

        service.cache.write().await.insert(
            vec![admin_role_id],
            CachedPermissions {
                codes: HashSet::new(),
                loaded_at: Instant::now(),
            },
        );

        assert!(
            !service
                .has_permission(
                    &[admin_role_id],
                    &permission_code,
                    PermissionCheckStrategy::Cached,
                )
                .await
                .unwrap()
        );
        assert!(
            service
                .has_permission(
                    &[admin_role_id],
                    &permission_code,
                    PermissionCheckStrategy::Direct,
                )
                .await
                .unwrap()
        );

        service
            .cache
            .write()
            .await
            .get_mut(&vec![admin_role_id])
            .unwrap()
            .loaded_at = Instant::now() - CACHE_TTL;
        assert!(
            service
                .has_permission(
                    &[admin_role_id],
                    &permission_code,
                    PermissionCheckStrategy::Cached,
                )
                .await
                .unwrap()
        );
    }
}
