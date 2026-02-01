use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::Rng;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::user::traits::PasswordHasher as DomainPasswordHasher;

/// Argon2id password hasher implementation using OWASP recommended settings.
#[derive(Debug, Clone)]
pub struct Argon2PasswordHasher;

impl Argon2PasswordHasher {
    pub fn new() -> Self {
        Self
    }

    fn create_argon2(&self) -> Argon2<'static> {
        let params = argon2::Params::new(
            65536, // 64MB memory
            3,     // 3 iterations
            4,     // 4 parallelism
            Some(32),
        )
        .expect("Invalid Argon2 params");

        Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
    }

    fn generate_salt(&self) -> SaltString {
        let mut salt = [0u8; 32];
        rand::rngs::OsRng.fill(&mut salt);
        // Convert to hex string (no padding)
        let salt_hex = hex::encode(salt);
        SaltString::from_b64(&salt_hex).expect("Failed to create salt string")
    }
}

impl Default for Argon2PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainPasswordHasher for Argon2PasswordHasher {
    fn hash_password(&self, password: &str) -> Result<String, DomainError> {
        let salt = self.generate_salt();
        let argon2 = self.create_argon2();

        let password_bytes = password.as_bytes();
        let hash_result = argon2.hash_password(password_bytes, &salt);

        match hash_result {
            Ok(hash) => Ok(hash.to_string()),
            Err(e) => {
                tracing::error!(error = %e, "Failed to hash password");
                Err(DomainError::Internal)
            }
        }
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse password hash");
            DomainError::Internal
        })?;

        let password_bytes = password.as_bytes();
        let argon2 = self.create_argon2();
        let verify_result = argon2.verify_password(password_bytes, &parsed_hash);

        match verify_result {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_returns_valid_hash() {
        let hasher = Argon2PasswordHasher::new();
        let password = "test_password_123";

        let hash = hasher.hash_password(password).expect("Should hash password");

        assert!(hash.starts_with("$argon2id$"), "Hash should start with argon2id marker");
        assert!(hash.contains("$v=19$"), "Hash should contain version marker");
        assert!(hash.contains("$m=65536,t=3,p=4$"), "Hash should contain parameters");
        assert!(hash.contains('$'), "Hash should contain salt separator");
    }

    #[test]
    fn test_verify_password_success() {
        let hasher = Argon2PasswordHasher::new();
        let password = "test_password_123";

        let hash = hasher.hash_password(password).expect("Should hash password");
        let result = hasher.verify_password(password, &hash).expect("Should verify password");

        assert!(result, "Password should verify successfully");
    }

    #[test]
    fn test_verify_password_failure() {
        let hasher = Argon2PasswordHasher::new();
        let password = "test_password_123";
        let wrong_password = "wrong_password";

        let hash = hasher.hash_password(password).expect("Should hash password");
        let result = hasher.verify_password(wrong_password, &hash).expect("Should verify password");

        assert!(!result, "Wrong password should not verify");
    }

    #[test]
    fn test_different_salts_produce_different_hashes() {
        let hasher = Argon2PasswordHasher::new();
        let password = "test_password_123";

        let hash1 = hasher.hash_password(password).expect("Should hash password");
        let hash2 = hasher.hash_password(password).expect("Should hash password");

        assert_ne!(hash1, hash2, "Same password should produce different hashes due to random salt");
    }

    #[test]
    fn test_hash_password_empty_password() {
        let hasher = Argon2PasswordHasher::new();
        let password = "";

        let result = hasher.hash_password(password);

        assert!(result.is_ok(), "Should handle empty password");
    }
}
