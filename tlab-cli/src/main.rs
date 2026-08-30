use std::{ffi::OsString, path::PathBuf, sync::Arc};

use anyhow::{Context, bail};
use serde::Deserialize;
use tlab::config::Loader;

#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    pub tracing: tlab::tracing::Config,
    pub http: tlab::http::Config,
}

pub struct CliContainer {
    pub config: CliConfig,
    pub http_server: tlab::http::HttpServer,
}

impl CliContainer {
    pub fn new(config: CliConfig) -> Self {
        Self {
            config: config.clone(),
            http_server: tlab::http::HttpServer::new(config.http.clone()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Loader::from_file(config_path(std::env::args_os().skip(1))?)
        .try_deserialize::<CliConfig>()?;
    tlab::tracing::initialize(&config.tracing)?;
    generate_self_signed_certificate_if_not_exist(&config)?;

    let container = Arc::new(CliContainer::new(config));

    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello, world!" }))
        .with_state(container.clone());

    container.http_server.run_https(router).await?;

    Ok(())
}

fn generate_self_signed_certificate_if_not_exist(config: &CliConfig) -> anyhow::Result<()> {
    // generate cert
    if let Some(tls_certificate_files) = &config.http.tls_certificate_files {
        if !std::path::Path::new(&tls_certificate_files.cert).exists()
            || !std::path::Path::new(&tls_certificate_files.key).exists()
        {
            let subject_alt_names = vec![config.http.host.clone()];
            tlab::cert::generate_self_signed_certificate(
                &subject_alt_names,
                tls_certificate_files.clone(),
            )?;
            tracing::info!("Certificate generated at {:?}", tls_certificate_files);
        }
    }
    Ok(())
}

fn config_path(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<PathBuf> {
    let mut args = args.into_iter();

    let path = match args.next() {
        Some(arg) if arg == "--config" => args
            .next()
            .map(PathBuf::from)
            .context("missing path after --config")?,
        Some(arg) => bail!("unexpected argument `{}`", arg.to_string_lossy()),
        None => default_config_path()?,
    };

    if let Some(arg) = args.next() {
        bail!("unexpected argument `{}`", arg.to_string_lossy());
    }

    Ok(path)
}

fn default_config_path() -> anyhow::Result<PathBuf> {
    let executable = std::env::current_exe().context("failed to determine executable path")?;
    let directory = executable
        .parent()
        .context("executable path has no parent directory")?;
    println!("directory: {:?}", directory.display());
    Ok(directory.join("config.yml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_path() {
        let path = config_path([OsString::from("--config"), OsString::from("custom.yml")]).unwrap();

        assert_eq!(path, PathBuf::from("custom.yml"));
    }

    #[test]
    fn rejects_config_flag_without_path() {
        assert!(config_path([OsString::from("--config")]).is_err());
    }
}
