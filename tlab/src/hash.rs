use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{
        PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString, rand_core::OsRng,
    },
};
use pbkdf2::{Algorithm as Pbkdf2Algorithm, Params as Pbkdf2Params, Pbkdf2};
use serde::Deserialize;

fn hash_error(error: impl std::fmt::Display) -> crate::Error {
    crate::Error::Internal(anyhow::anyhow!(error.to_string()))
}

#[derive(Debug, Clone, Deserialize)]
pub struct Argon2Config {
    pub memory_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub output_len: Option<usize>,
}

pub struct Argon2PasswordHasher {
    params: Params,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pbkdf2Config {
    pub iterations: u32,
    pub output_len: usize,
}

pub struct Pbkdf2PasswordHasher {
    params: Pbkdf2Params,
}

impl Pbkdf2PasswordHasher {
    pub fn new(config: &Pbkdf2Config) -> Self {
        Self {
            params: Pbkdf2Params {
                rounds: config.iterations,
                output_length: config.output_len,
            },
        }
    }
}

impl Argon2PasswordHasher {
    pub fn new(config: &Argon2Config) -> crate::Result<Self> {
        let params = Params::new(
            config.memory_cost_kib,
            config.time_cost,
            config.parallelism,
            config.output_len,
        )
        .map_err(hash_error)?;
        Ok(Self { params })
    }

    fn argon2(&self) -> Argon2<'_> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
    }
}

pub trait PasswordHasher : Sync + Send {
    fn hash(&self, password: &str) -> crate::Result<String>;
    fn verify(&self, password: &str, hashed: &str) -> crate::Result<()>;
    fn needs_rehash(&self, hashed: &str) -> crate::Result<bool>;
}

impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, password: &str) -> crate::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        Ok(self
            .argon2()
            .hash_password(password.as_bytes(), &salt)
            .map_err(hash_error)?
            .to_string())
    }

    fn verify(&self, password: &str, hashed: &str) -> crate::Result<()> {
        let hash = PasswordHash::new(hashed).map_err(hash_error)?;
        self.argon2()
            .verify_password(password.as_bytes(), &hash)
            .map_err(hash_error)?;
        Ok(())
    }

    fn needs_rehash(&self, hashed: &str) -> crate::Result<bool> {
        let hash = PasswordHash::new(hashed).map_err(hash_error)?;
        let params = Params::try_from(&hash).map_err(hash_error)?;
        Ok(hash.algorithm != Algorithm::Argon2id.ident()
            || hash.version != Some(Version::V0x13.into())
            || params != self.params)
    }
}

impl PasswordHasher for Pbkdf2PasswordHasher {
    fn hash(&self, password: &str) -> crate::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        Ok(Pbkdf2
            .hash_password_customized(password.as_bytes(), None, None, self.params, salt.as_salt())
            .map_err(hash_error)?
            .to_string())
    }

    fn verify(&self, password: &str, hashed: &str) -> crate::Result<()> {
        let hash = PasswordHash::new(hashed).map_err(hash_error)?;
        Pbkdf2
            .verify_password(password.as_bytes(), &hash)
            .map_err(hash_error)?;
        Ok(())
    }

    fn needs_rehash(&self, hashed: &str) -> crate::Result<bool> {
        let hash = PasswordHash::new(hashed).map_err(hash_error)?;
        let params = Pbkdf2Params::try_from(&hash).map_err(hash_error)?;
        Ok(hash.algorithm != Pbkdf2Algorithm::Pbkdf2Sha256.ident() || params != self.params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Argon2Config {
        Argon2Config {
            memory_cost_kib: 19_456,
            time_cost: 2,
            parallelism: 1,
            output_len: Some(32),
        }
    }

    #[test]
    fn hashes_and_verifies_a_password() {
        let hasher = Argon2PasswordHasher::new(&config()).unwrap();
        let hash = hasher.hash("password").unwrap();

        hasher.verify("password", &hash).unwrap();
        assert!(hasher.verify("incorrect", &hash).is_err());
    }

    #[test]
    fn identifies_a_hash_that_needs_rehashing() {
        let hasher = Argon2PasswordHasher::new(&config()).unwrap();
        let hash = hasher.hash("password").unwrap();
        assert!(!hasher.needs_rehash(&hash).unwrap());

        let stronger = Argon2PasswordHasher::new(&Argon2Config {
            time_cost: 3,
            ..config()
        })
        .unwrap();
        assert!(stronger.needs_rehash(&hash).unwrap());
    }

    #[test]
    fn pbkdf2_hashes_and_identifies_a_hash_that_needs_rehashing() {
        let hasher = Pbkdf2PasswordHasher::new(&Pbkdf2Config {
            iterations: 1_000,
            output_len: 32,
        });
        let hash = hasher.hash("password").unwrap();

        hasher.verify("password", &hash).unwrap();
        assert!(hasher.verify("incorrect", &hash).is_err());
        assert!(!hasher.needs_rehash(&hash).unwrap());

        let stronger = Pbkdf2PasswordHasher::new(&Pbkdf2Config {
            iterations: 2_000,
            output_len: 32,
        });
        assert!(stronger.needs_rehash(&hash).unwrap());
    }
}
