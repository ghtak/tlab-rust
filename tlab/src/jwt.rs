use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct EdDsaKeyFiles {
    pub private_key: String,
    pub public_key: String,
    pub generate_if_missing: bool,
}

impl EdDsaKeyFiles {
    pub fn ensure_files(&self) -> Result<()> {
        self.validate_paths()?;
        match (
            Path::new(&self.private_key).exists(),
            Path::new(&self.public_key).exists(),
        ) {
            (true, true) => Ok(()),
            (false, false) if self.generate_if_missing => self.generate(),
            (false, false) => Err(Error::IllegalState("JWT key files are missing".into())),
            _ => Err(Error::IllegalState("JWT key files are incomplete".into())),
        }
    }

    pub fn generate(&self) -> Result<()> {
        self.validate_paths()?;
        if Path::new(&self.private_key).exists() || Path::new(&self.public_key).exists() {
            return Err(Error::Conflict("JWT key file already exists".into()));
        }

        let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519)
            .map_err(|error| Error::Internal(anyhow::anyhow!(error)))?;

        let mut private_options = OpenOptions::new();
        private_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            private_options.mode(0o600);
        }
        private_options
            .open(&self.private_key)?
            .write_all(key_pair.serialize_pem().as_bytes())?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.public_key)?
            .write_all(key_pair.public_key_pem().as_bytes())?;
        Ok(())
    }

    fn validate_paths(&self) -> Result<()> {
        if self.private_key.trim().is_empty()
            || self.public_key.trim().is_empty()
            || std::path::absolute(&self.private_key)? == std::path::absolute(&self.public_key)?
        {
            return Err(Error::IllegalState("invalid JWT key file paths".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub key_files: EdDsaKeyFiles,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access: IssuedToken,
    pub refresh: IssuedToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenUse {
    Access,
    Refresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
    pub token_use: TokenUse,
}

pub struct JwtCodec {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    audience: String,
    access_token_ttl_seconds: u64,
    refresh_token_ttl_seconds: u64,
}

impl JwtCodec {
    pub fn new(config: &JwtConfig) -> Result<Self> {
        if config.issuer.trim().is_empty()
            || config.audience.trim().is_empty()
            || config.access_token_ttl_seconds == 0
            || config.refresh_token_ttl_seconds == 0
        {
            return Err(Error::IllegalState("invalid JWT configuration".into()));
        }

        config.key_files.ensure_files()?;

        let private_key = fs::read(&config.key_files.private_key)?;
        let public_key = fs::read(&config.key_files.public_key)?;
        let encoding_key = EncodingKey::from_ed_pem(&private_key)
            .map_err(|error| Error::Internal(anyhow::anyhow!(error)))?;
        let decoding_key = DecodingKey::from_ed_pem(&public_key)
            .map_err(|error| Error::Internal(anyhow::anyhow!(error)))?;

        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.leeway = 0;
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);

        let codec = Self {
            encoding_key,
            decoding_key,
            validation,
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            access_token_ttl_seconds: config.access_token_ttl_seconds,
            refresh_token_ttl_seconds: config.refresh_token_ttl_seconds,
        };
        let probe = codec.issue("jwt-key-check", TokenUse::Access, now()?, 60)?;
        codec
            .verify(&probe.token)
            .map_err(|_| Error::IllegalState("JWT key pair does not match".into()))?;
        Ok(codec)
    }

    pub fn issue_pair(&self, subject: &str) -> Result<TokenPair> {
        if subject.is_empty() {
            return Err(Error::IllegalState("JWT subject is empty".into()));
        }
        let issued_at = now()?;
        let access = self.issue(
            subject,
            TokenUse::Access,
            issued_at,
            self.access_token_ttl_seconds,
        )?;
        let refresh = self.issue(
            subject,
            TokenUse::Refresh,
            issued_at,
            self.refresh_token_ttl_seconds,
        )?;
        Ok(TokenPair { access, refresh })
    }

    pub fn verify(&self, token: &str) -> Result<JwtClaims> {
        let claims = jsonwebtoken::decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|_| Error::InvalidToken)?
            .claims;
        if claims.sub.is_empty() || claims.exp <= claims.iat || claims.iat > now()? {
            return Err(Error::InvalidToken);
        }
        Ok(claims)
    }

    fn issue(&self, subject: &str, token_use: TokenUse, iat: u64, ttl: u64) -> Result<IssuedToken> {
        let exp = iat
            .checked_add(ttl)
            .ok_or_else(|| Error::IllegalState("JWT expiration overflow".into()))?;
        let claims = JwtClaims {
            sub: subject.to_owned(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat,
            exp,
            token_use,
        };
        let token =
            jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), &claims, &self.encoding_key)
                .map_err(|error| Error::Internal(anyhow::anyhow!(error)))?;
        Ok(IssuedToken {
            token,
            expires_at: exp,
        })
    }
}

fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Internal(anyhow::anyhow!(error)))?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct TestKeys {
        directory: PathBuf,
    }

    impl TestKeys {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("tlab-jwt-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&directory).unwrap();
            Self { directory }
        }

        fn config(&self) -> JwtConfig {
            JwtConfig {
                key_files: EdDsaKeyFiles {
                    private_key: self
                        .directory
                        .join("private.pem")
                        .to_string_lossy()
                        .into_owned(),
                    public_key: self
                        .directory
                        .join("public.pem")
                        .to_string_lossy()
                        .into_owned(),
                    generate_if_missing: true,
                },
                issuer: "tlab".into(),
                audience: "tlab-api".into(),
                access_token_ttl_seconds: 60,
                refresh_token_ttl_seconds: 3600,
            }
        }
    }

    impl Drop for TestKeys {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.directory.join("private.pem"));
            let _ = fs::remove_file(self.directory.join("public.pem"));
            let _ = fs::remove_dir(&self.directory);
        }
    }

    #[test]
    fn generates_keys_and_verifies_both_tokens_after_reload() {
        let keys = TestKeys::new();
        let config = keys.config();
        let codec = JwtCodec::new(&config).unwrap();
        let pair = codec.issue_pair("user-42").unwrap();

        assert_ne!(pair.access.token, pair.refresh.token);
        assert!(
            fs::read_to_string(&config.key_files.private_key)
                .unwrap()
                .starts_with("-----BEGIN PRIVATE KEY-----")
        );
        assert!(
            fs::read_to_string(&config.key_files.public_key)
                .unwrap()
                .starts_with("-----BEGIN PUBLIC KEY-----")
        );

        let mut reload_config = config.clone();
        reload_config.key_files.generate_if_missing = false;
        let reloaded = JwtCodec::new(&reload_config).unwrap();
        let access = reloaded.verify(&pair.access.token).unwrap();
        let refresh = reloaded.verify(&pair.refresh.token).unwrap();
        assert_eq!(access.sub, "user-42");
        assert_eq!(access.token_use, TokenUse::Access);
        assert_eq!(pair.access.expires_at, access.exp);
        assert_eq!(access.exp - access.iat, 60);
        assert_eq!(refresh.sub, "user-42");
        assert_eq!(refresh.token_use, TokenUse::Refresh);
        assert_eq!(pair.refresh.expires_at, refresh.exp);
        assert_eq!(refresh.exp - refresh.iat, 3600);
        assert!(matches!(
            config.key_files.generate(),
            Err(Error::Conflict(_))
        ));
    }

    #[test]
    fn rejects_missing_or_incomplete_key_files() {
        let keys = TestKeys::new();
        let mut config = keys.config();
        config.key_files.generate_if_missing = false;
        assert!(matches!(
            JwtCodec::new(&config),
            Err(Error::IllegalState(_))
        ));

        fs::write(&config.key_files.private_key, "incomplete key").unwrap();
        config.key_files.generate_if_missing = true;
        assert!(matches!(
            JwtCodec::new(&config),
            Err(Error::IllegalState(_))
        ));
    }

    #[test]
    fn rejects_invalid_signature_expiration_issuer_and_audience() {
        let keys = TestKeys::new();
        let config = keys.config();
        let codec = JwtCodec::new(&config).unwrap();
        let token = codec.issue_pair("user-42").unwrap().access.token;

        let mut parts = token.split('.');
        let header = parts.next().unwrap();
        let payload = parts.next().unwrap();
        let signature = parts.next().unwrap();
        let first = if signature.starts_with('A') { 'B' } else { 'A' };
        let tampered = format!("{header}.{payload}.{first}{}", &signature[1..]);
        assert!(matches!(codec.verify(&tampered), Err(Error::InvalidToken)));

        let expired = codec
            .issue("user-42", TokenUse::Access, now().unwrap() - 100, 1)
            .unwrap();
        assert!(matches!(
            codec.verify(&expired.token),
            Err(Error::InvalidToken)
        ));

        let other_keys = TestKeys::new();
        let other_codec = JwtCodec::new(&other_keys.config()).unwrap();
        assert!(matches!(
            other_codec.verify(&token),
            Err(Error::InvalidToken)
        ));

        let mut mismatched_key_files = config.clone();
        mismatched_key_files.key_files.public_key = other_keys.config().key_files.public_key;
        assert!(matches!(
            JwtCodec::new(&mismatched_key_files),
            Err(Error::IllegalState(_))
        ));

        let claims = codec.verify(&token).unwrap();
        let wrong_algorithm = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"another secret"),
        )
        .unwrap();
        assert!(matches!(
            codec.verify(&wrong_algorithm),
            Err(Error::InvalidToken)
        ));

        let mut other_issuer = config.clone();
        other_issuer.issuer = "another issuer".into();
        assert!(matches!(
            JwtCodec::new(&other_issuer).unwrap().verify(&token),
            Err(Error::InvalidToken)
        ));

        let mut other_audience = config.clone();
        other_audience.audience = "another audience".into();
        assert!(matches!(
            JwtCodec::new(&other_audience).unwrap().verify(&token),
            Err(Error::InvalidToken)
        ));
    }
}
