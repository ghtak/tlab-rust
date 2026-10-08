#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(serde::Deserialize, Debug)]
struct GoogleTokenResponse {
    id_token: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct GoogleIdTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
}

pub fn get_auth_url(config: &Config, state: &str) -> url::Url {
    let mut url = url::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url
}

pub async fn handle_oauth2_callback(
    config: &Config,
    code: &str,
) -> crate::Result<GoogleIdTokenClaims> {
    let params = [
        ("code", code),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
        ("redirect_uri", &config.redirect_uri),
        ("grant_type", "authorization_code"),
    ];
    let res = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| crate::error::Error::Internal(e.into()))?
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|e| crate::error::Error::Internal(e.into()))?;

    let client = google_oauth::AsyncClient::new(config.client_id.clone());
    let id_token = client.validate_id_token(res.id_token).await.map_err(|e| {
        crate::error::Error::Internal(anyhow::anyhow!(e).context("id_token decode failed"))
    })?;
    Ok(GoogleIdTokenClaims {
        sub: id_token.sub,
        email: id_token.email,
        email_verified: id_token.email_verified == Some(true),
        name: id_token.name,
    })
}
