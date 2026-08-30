use std::time::Duration;

use anyhow::Context;

use tracing::info;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub tls_certificate_files: Option<TlsCertificateFiles>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct TlsCertificateFiles {
    pub cert: String,
    pub key: String,
}

pub struct HttpServer {
    config: Config,
    tls_config: tokio::sync::OnceCell<axum_server::tls_rustls::RustlsConfig>,
}

impl HttpServer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tls_config: tokio::sync::OnceCell::new(),
        }
    }

    pub async fn run_http(&self, app: axum::Router) -> crate::Result<()> {
        let listener = self.listener().await?;
        let address = listener
            .local_addr()
            .context("failed to read bound listener address")?;

        info!(%address, "HTTP server listening");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("HTTP server terminated unexpectedly")?;

        info!("HTTP server stopped");
        Ok(())
    }

    pub async fn run_https(&self, app: axum::Router) -> crate::Result<()> {
        let tls_config = self.load_tls_config().await?;
        let listener = self.listener().await?;
        let address = listener
            .local_addr()
            .context("failed to read bound listener address")?;

        info!(%address, "HTTPS server listening");

        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.graceful_shutdown(Some(Duration::from_secs(30)));
        });

        axum_server::from_tcp_rustls(
            listener
                .into_std()
                .context("failed to convert HTTPS listener for TLS serving")?,
            tls_config.clone(),
        )
        .context("failed to configure HTTPS server")?
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("HTTPS server terminated unexpectedly")?;

        info!("HTTPS server stopped");
        Ok(())
    }

    async fn listener(&self) -> crate::Result<tokio::net::TcpListener> {
        let listener = tokio::net::TcpListener::bind((self.config.host.as_str(), self.config.port))
            .await
            .with_context(|| {
                format!(
                    "failed to bind listener to {}:{}",
                    self.config.host, self.config.port
                )
            })?;

        Ok(listener)
    }

    async fn load_tls_config(&self) -> crate::Result<&axum_server::tls_rustls::RustlsConfig> {
        let Some(tls_certificate_files) = self.config.tls_certificate_files.as_ref() else {
            return Err(
                anyhow::anyhow!("TLS configuration is required to run an HTTPS server").into(),
            );
        };

        self.tls_config
            .get_or_try_init(|| async {
                axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    &tls_certificate_files.cert,
                    &tls_certificate_files.key,
                )
                .await
                .context("failed to load TLS certificate and key")
                .map_err(Into::into)
            })
            .await
    }

    pub async fn reload_tls_config(
        &self,
        tls_certificate_files: &TlsCertificateFiles,
    ) -> crate::Result<()> {
        let Some(tls_config) = self.tls_config.get() else {
            return Err(anyhow::anyhow!("TLS configuration has not been initialized").into());
        };

        tls_config
            .reload_from_pem_file(&tls_certificate_files.cert, &tls_certificate_files.key)
            .await
            .context("failed to reload TLS certificate and key")?;

        info!("TLS certificate and key reloaded");
        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "failed to listen for Ctrl-C");
        }
    };
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let terminate = async {
            match signal(SignalKind::terminate()) {
                Ok(mut stream) => {
                    stream.recv().await;
                }
                Err(error) => tracing::warn!(%error, "failed to listen for SIGTERM"),
            }
        };

        tokio::select! {
            () = ctrl_c => {},
            () = terminate => {},
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await;

    info!("shutdown signal received; graceful shutdown started");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(tls_certificate_files: Option<TlsCertificateFiles>) -> Config {
        Config {
            host: "127.0.0.1".to_owned(),
            port: 0,
            tls_certificate_files,
        }
    }

    #[tokio::test]
    async fn http_can_bind_without_tls() {
        let server = HttpServer::new(config(None));

        let listener = server.listener().await.unwrap();

        assert_eq!(listener.local_addr().unwrap().ip().to_string(), "127.0.0.1");
    }

    #[tokio::test]
    async fn https_requires_tls_configuration() {
        let server = HttpServer::new(config(None));

        let error = server.run_https(axum::Router::new()).await.unwrap_err();

        assert_eq!(
            error.to_string(),
            "internal error: TLS configuration is required to run an HTTPS server"
        );
    }

    #[tokio::test]
    async fn reloading_uninitialized_tls_config_fails() {
        let server = HttpServer::new(config(None));
        let tls_certificate_files = TlsCertificateFiles {
            cert: "cert.pem".to_owned(),
            key: "key.pem".to_owned(),
        };

        let error = server
            .reload_tls_config(&tls_certificate_files)
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "internal error: TLS configuration has not been initialized"
        );
    }
}
