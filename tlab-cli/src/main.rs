use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;
use tlab::config::Loader;
use tlab::tracing::Config;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    pub tracing: Config,
}

fn main() -> anyhow::Result<()> {
    let config = Loader::from_file(config_path(std::env::args_os().skip(1))?)
        .try_deserialize::<CliConfig>()?;
    println!("{:#?}", config);

    tlab::tracing::initialize(&config.tracing)?;

    info!("Hello, world!");
    warn!("This is a warning");
    error!("This is an error");

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
