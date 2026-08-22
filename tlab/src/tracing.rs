//! Process-wide tracing subscriber initialization.

use std::sync::OnceLock;

use anyhow::Context;
use tracing_appender::{non_blocking::WorkerGuard, rolling::daily};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::config::TracingConfig;

static LOGGING_GUARDS: OnceLock<Vec<WorkerGuard>> = OnceLock::new();

/// Installs App's process-wide tracing subscriber.
///
/// If `RUST_LOG` is set, its valid value overrides the filters for every
/// configured output. Otherwise each output's configured filter is used.
/// Calling this more than once, or after another global subscriber has been
/// installed, returns an error.
pub fn initialize(tracing_config: &TracingConfig) -> crate::Result<()> {
    if tracing_config.console.is_none() && tracing_config.file.is_none() {
        return Ok(());
    }

    let mut guards = Vec::new();

    let console_layer = tracing_config
        .console
        .as_ref()
        .map(|config| -> anyhow::Result<_> {
            let (writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
                .buffered_lines_limit(config.buffered_lines_limit)
                .lossy(config.lossy)
                .finish(std::io::stderr());
            guards.push(guard);

            Ok(fmt::layer()
                .with_writer(writer)
                .compact()
                .with_ansi(config.ansi)
                .with_filter(resolve_filter(&config.filter)?))
        })
        .transpose()?;

    let file_layer = tracing_config
        .file
        .as_ref()
        .map(|config| -> anyhow::Result<_> {
            std::fs::create_dir_all(&config.directory).with_context(|| {
                format!(
                    "failed to create log directory `{}`",
                    config.directory.display()
                )
            })?;
            let (writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
                .buffered_lines_limit(config.buffered_lines_limit)
                .lossy(config.lossy)
                .finish(daily(&config.directory, &config.filename));
            guards.push(guard);

            Ok(fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_filter(resolve_filter(&config.filter)?))
        })
        .transpose()?;

    Registry::default()
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .context("failed to install the global tracing subscriber")?;

    LOGGING_GUARDS
        .set(guards)
        .map_err(|_| anyhow::anyhow!("tracing worker guards were already installed"))?;
    Ok(())
}

fn resolve_filter(configured_filter: &str) -> anyhow::Result<EnvFilter> {
    if std::env::var_os(EnvFilter::DEFAULT_ENV).is_some() {
        EnvFilter::try_from_default_env().context("invalid RUST_LOG filter")
    } else {
        EnvFilter::try_new(configured_filter)
            .with_context(|| format!("invalid tracing filter `{configured_filter}`"))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        sync::{Mutex, OnceLock},
    };

    use super::*;
    use crate::config::ConsoleTraceConfig;

    fn environment_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvironmentVariable {
        name: &'static str,
        previous: Option<OsString>,
    }

    impl EnvironmentVariable {
        fn set(name: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(name);
            unsafe {
                std::env::set_var(name, value);
            }
            Self { name, previous }
        }

        fn remove(name: &'static str) -> Self {
            let previous = std::env::var_os(name);
            unsafe {
                std::env::remove_var(name);
            }
            Self { name, previous }
        }
    }

    impl Drop for EnvironmentVariable {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = &self.previous {
                    std::env::set_var(self.name, value);
                } else {
                    std::env::remove_var(self.name);
                }
            }
        }
    }

    #[test]
    fn initializes_without_outputs() {
        initialize(&TracingConfig::default()).unwrap();
    }

    #[test]
    fn rejects_invalid_configured_filter() {
        let _lock = environment_lock().lock().unwrap();
        let _rust_log = EnvironmentVariable::remove(EnvFilter::DEFAULT_ENV);
        let config = TracingConfig {
            console: Some(ConsoleTraceConfig {
                filter: "[".into(),
                buffered_lines_limit: 1,
                lossy: true,
                ansi: false,
            }),
            file: None,
        };

        assert!(initialize(&config).is_err());
    }

    #[test]
    fn rust_log_overrides_the_configured_filter() {
        let _lock = environment_lock().lock().unwrap();
        let _rust_log = EnvironmentVariable::set(EnvFilter::DEFAULT_ENV, "[");

        assert!(resolve_filter("info").is_err());
    }
}
