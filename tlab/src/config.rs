use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, de::DeserializeOwned};

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

#[derive(Debug, Clone, Deserialize)]
pub struct TlabConfig {
    pub tracing: TracingConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TracingConfig {
    pub console: Option<ConsoleTraceConfig>,
    pub file: Option<FileTraceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsoleTraceConfig {
    pub filter: String,
    pub buffered_lines_limit: usize,
    pub lossy: bool,
    pub ansi: bool,
}

/// File tracing settings.
#[derive(Debug, Clone, Deserialize)]
pub struct FileTraceConfig {
    pub directory: PathBuf,
    pub filename: String,
    pub filter: String,
    pub buffered_lines_limit: usize,
    pub lossy: bool,
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, time::SystemTime};

    use super::*;

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

    struct EnvironmentVariable {
        name: String,
        previous: Option<OsString>,
    }

    impl EnvironmentVariable {
        fn set(name: String, value: &str) -> Self {
            let previous = std::env::var_os(&name);
            unsafe {
                std::env::set_var(&name, value);
            }
            Self { name, previous }
        }
    }

    impl Drop for EnvironmentVariable {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = &self.previous {
                    std::env::set_var(&self.name, value);
                } else {
                    std::env::remove_var(&self.name);
                }
            }
        }
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

        let config: TlabConfig = Loader::from_file(path.clone()).try_deserialize().unwrap();

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
