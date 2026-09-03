use std::path::PathBuf;

use serde::Deserialize;
use tlab::config::Loader;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub tracing: tlab::tracing::Config,
    pub http: tlab::http::Config,
    pub tls_certificate_files: Option<tlab::cert::TlsCertificateFiles>,
}

impl AppConfig {
    pub fn load() -> tlab::Result<AppConfig> {
        let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();
        let config_file_name = PathBuf::from("config.yml");
        let config_file_full_path = if cargo_manifest_dir.is_some() {
            PathBuf::from(cargo_manifest_dir.unwrap()).join(config_file_name)
        } else {
            config_file_name
        };

        let config = Loader::from_file(config_file_full_path).try_deserialize::<AppConfig>()?;
        Ok(config)
    }
}
