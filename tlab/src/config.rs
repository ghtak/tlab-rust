use std::path::PathBuf;

use anyhow::Context;
use serde::de::DeserializeOwned;

pub struct Loader {
    file: Option<PathBuf>,
    env_prefix: Option<String>,
}

impl Loader {
    pub fn from_file(path: PathBuf) -> Self {
        Self {
            file: Some(path),
            env_prefix: None,
        }
    }

    pub fn env_prefix(mut self, env_prefix: impl Into<String>) -> Self {
        self.env_prefix = Some(env_prefix.into());
        self
    }

    pub fn load(self) -> crate::Result<config::Config> {
        let mut builder = config::Config::builder();

        if let Some(path) = self.file {
            builder = builder.add_source(config::File::from(path));
        }

        if let Some(prefix) = self.env_prefix {
            builder = builder.add_source(
                config::Environment::with_prefix(&prefix)
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true),
            );
        }

        builder
            .build()
            .map_err(|e| crate::Error::Internal(e.into()))
    }

    pub fn try_deserialize<T>(self) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        let config = self.load()?;
        config
            .try_deserialize()
            .context("failed to deserialize config")
            .map_err(crate::Error::Internal)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;
    use crate::test_support::{EnvironmentVariable, environment_lock};

    fn config_path(extension: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "tlab-config-{}-{timestamp}.{extension}",
            std::process::id()
        ))
    }

    #[test]
    fn loads_values_from_file() {
        let path = config_path("toml");
        fs::write(&path, "name = \"tlab\"\n").unwrap();

        let config = Loader::from_file(path.clone()).load().unwrap();

        assert_eq!(config.get_string("name").unwrap(), "tlab");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_create_a_missing_file() {
        let path = config_path("toml");

        assert!(Loader::from_file(path.clone()).load().is_err());
        assert!(!path.exists());
    }

    #[test]
    fn loads_values_from_environment_prefix() {
        let _lock = environment_lock().lock().unwrap();
        let prefix = format!("TLAB_CONFIG_TEST_{}", std::process::id());
        let key = format!("{prefix}_TRACING__CONSOLE__FILTER");
        let _filter = EnvironmentVariable::set(key, "debug");

        let config = Loader {
            file: None,
            env_prefix: None,
        }
        .env_prefix(&prefix)
        .load()
        .unwrap();

        assert_eq!(
            config.get_string("tracing.console.filter").unwrap(),
            "debug"
        );
    }

    #[test]
    fn deserializes_tlab_config_from_yaml_file() {
        let path = config_path("yaml");
        fs::write(
            &path,
            r#"
tracing:
  console:
    filter: info
    buffered_lines_limit: 100
    lossy: true
    ansi: false
  file:
    directory: logs
    filename: tlab.log
    filter: warn
    buffered_lines_limit: 200
    lossy: false
"#,
        )
        .unwrap();

        #[derive(Debug, Clone, serde::Deserialize)]
        pub struct TestConfig {
            pub tracing: crate::tracing::Config,
        }

        let config: TestConfig = Loader::from_file(path.clone()).try_deserialize().unwrap();

        let console = config.tracing.console.unwrap();
        assert_eq!(console.filter, "info");
        assert_eq!(console.buffered_lines_limit, 100);
        assert!(console.lossy);
        assert!(!console.ansi);

        let file = config.tracing.file.unwrap();
        assert_eq!(file.directory, PathBuf::from("logs"));
        assert_eq!(file.filename, "tlab.log");
        assert_eq!(file.filter, "warn");
        assert_eq!(file.buffered_lines_limit, 200);
        assert!(!file.lossy);

        fs::remove_file(path).unwrap();
    }
}
