use std::fs::File;
use std::io::Write;

use rcgen::{CertifiedKey, generate_simple_self_signed};

use crate::http::TlsCertificateFiles;

pub fn generate_self_signed_certificate(
    subject_alt_names: &[String],
    tlsfile: TlsCertificateFiles,
) -> crate::Result<()> {
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| crate::Error::Internal(anyhow::anyhow!(e)))?;

    let cert_path = tlsfile.cert;
    let key_path = tlsfile.key;

    if std::path::Path::new(&cert_path).exists() {
        return Err(crate::Error::Conflict(
            "Certificate file already exists".into(),
        ));
    }

    File::create(cert_path)?.write_all(cert.pem().as_bytes())?;
    File::create(key_path)?.write_all(signing_key.serialize_pem().as_bytes())?;
    Ok(())
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
        let result = generate_self_signed_certificate(
            &subject_alt_names,
            TlsCertificateFiles {
                cert: "test_cert.pem".to_string(),
                key: "test_key.pem".to_string(),
            },
        );
        assert!(result.is_ok());

        // Verify that the files were created.
        assert!(std::path::Path::new("test_cert.pem").exists());
        assert!(std::path::Path::new("test_key.pem").exists());

        let result = generate_self_signed_certificate(
            &subject_alt_names,
            TlsCertificateFiles {
                cert: "test_cert.pem".to_string(),
                key: "test_key.pem".to_string(),
            },
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(crate::Error::Conflict(_))));

        // Clean up.
        std::fs::remove_file("test_cert.pem")?;
        std::fs::remove_file("test_key.pem")?;

        Ok(())
    }
}
