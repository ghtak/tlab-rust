use std::fs::OpenOptions;
use std::io::Write;

use rcgen::{CertifiedKey, generate_simple_self_signed};

#[derive(Debug, serde::Deserialize, Clone)]
pub struct TlsCertificateFiles {
    pub cert: String,
    pub key: String,
    pub generate_if_missing: bool,
}

impl TlsCertificateFiles {
    pub fn exists(&self) -> bool {
        std::path::Path::new(&self.cert).exists() || std::path::Path::new(&self.key).exists()
    }

    pub fn ensure_files(&self, subject_alt_names: &[String]) -> crate::Result<()> {
        match (
            std::path::Path::new(&self.cert).exists(),
            std::path::Path::new(&self.key).exists(),
        ) {
            (true, true) => Ok(()),
            (false, false) if self.generate_if_missing => {
                self.generate_self_signed_certificate(subject_alt_names)
            }
            (false, false) => Err(crate::Error::IllegalState(
                "TLS certificate files are missing".into(),
            )),
            _ => Err(crate::Error::IllegalState(
                "TLS certificate files are incomplete".into(),
            )),
        }
    }

    pub fn generate_self_signed_certificate(
        &self,
        subject_alt_names: &[String],
    ) -> crate::Result<()> {
        if self.cert.is_empty() || self.key.is_empty() {
            return Err(crate::Error::IllegalState(
                "Certificate file path is empty".into(),
            ));
        }

        let cert_path = std::path::Path::new(&self.cert);
        let key_path = std::path::Path::new(&self.key);

        if self.exists() {
            return Err(crate::Error::Conflict(
                "Certificate file already exists".into(),
            ));
        }

        let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
            .map_err(|e| crate::Error::Internal(anyhow::anyhow!(e)))?;

        let mut private_options = OpenOptions::new();
        private_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            private_options.mode(0o600);
        }
        private_options
            .open(key_path)?
            .write_all(signing_key.serialize_pem().as_bytes())?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(cert_path)?
            .write_all(cert.pem().as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed_certificate() -> Result<(), Box<dyn std::error::Error>> {
        // Clean up.
        let _ = std::fs::remove_file("test_cert.pem");
        let _ = std::fs::remove_file("test_key.pem");

        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let tls_certificate_files = TlsCertificateFiles {
            cert: "test_cert.pem".to_string(),
            key: "test_key.pem".to_string(),
            generate_if_missing: true,
        };
        let result = tls_certificate_files.ensure_files(&subject_alt_names);
        assert!(result.is_ok());

        // Verify that the files were created.
        assert!(tls_certificate_files.exists());
        assert!(std::path::Path::new("test_cert.pem").exists());
        assert!(std::path::Path::new("test_key.pem").exists());
        assert!(
            tls_certificate_files
                .ensure_files(&subject_alt_names)
                .is_ok()
        );

        let result = tls_certificate_files.generate_self_signed_certificate(&subject_alt_names);
        assert!(result.is_err());
        assert!(matches!(result, Err(crate::Error::Conflict(_))));

        // Clean up.
        std::fs::remove_file("test_cert.pem")?;
        std::fs::remove_file("test_key.pem")?;

        let mut no_generate = tls_certificate_files.clone();
        no_generate.generate_if_missing = false;
        assert!(matches!(
            no_generate.ensure_files(&subject_alt_names),
            Err(crate::Error::IllegalState(_))
        ));

        std::fs::write("test_key.pem", "partial key")?;
        assert!(matches!(
            tls_certificate_files.ensure_files(&subject_alt_names),
            Err(crate::Error::IllegalState(_))
        ));
        std::fs::remove_file("test_key.pem")?;

        Ok(())
    }
}
