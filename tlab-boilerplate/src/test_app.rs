use std::sync::Arc;

use axum::Router;

use crate::{app_config::AppConfig, app_container::AppContainer, auth, migration, test_db};

pub async fn setup() -> (Router, Arc<AppContainer>) {
    let mut config = AppConfig::load().unwrap();
    config.database = test_db::config();

    let key_dir = std::env::temp_dir().join(format!("tlab-test-jwt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&key_dir).unwrap();
    config.jwt.key_files.private_key = key_dir.join("private.pem").to_string_lossy().into_owned();
    config.jwt.key_files.public_key = key_dir.join("public.pem").to_string_lossy().into_owned();
    config.jwt.key_files.generate_if_missing = true;

    let container = Arc::new(AppContainer::new(config).await.unwrap());
    std::fs::remove_dir_all(key_dir).unwrap();
    migration::initialize_admin(&container.database, container.password_hasher.as_ref())
        .await
        .unwrap();

    let app = auth::route::router().with_state(container.clone());
    (app, container)
}
