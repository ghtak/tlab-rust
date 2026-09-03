mod app_config;

use app_config::AppConfig;

#[tokio::main]
async fn main() -> tlab::Result<()> {
    let config = AppConfig::load()?;
    tlab::tracing::initialize(&config.tracing)?;
    tracing::info!("{:#?}", config);

    let app = axum::Router::new().route("/", axum::routing::get(|| async { "Hello, world!" }));

    let http = tlab::http::Server::new(config.http.clone());

    if let Some(tls_certificate_files) = config.tls_certificate_files.as_ref() {
        if !tls_certificate_files.exists() {
            tracing::info!("Generating self-signed certificate");
            tls_certificate_files
                .generate_self_signed_certificate(&vec![config.http.host.clone()])?;
        }
        http.run_https(app, tls_certificate_files).await?;
    } else {
        http.run_http(app).await?;
    }

    Ok(())
}
