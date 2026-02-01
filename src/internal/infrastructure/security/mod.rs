//! Security infrastructure components for authentication and authorization.
//!
//! This module provides implementations for password hashing and JWT token generation.
//!
//! # Modules
//!
//! - [`password`] - Password hashing using Argon2id
//! - [`jwt`] - JWT token generation and validation
//!
//! # Re-exports
//!
//! The following implementations are re-exported for convenience:
//!
//! - [`Argon2PasswordHasher`] - Argon2id password hasher
//! - [`JwtGeneratorImpl`] - JWT token generator
//!

pub mod password;
pub mod jwt;

// Re-export implementations for easier access
pub use password::Argon2PasswordHasher;
pub use jwt::{Claims, JwtGeneratorImpl};

/// Create a default password hasher instance.
///
/// Returns an [`Argon2PasswordHasher`] with OWASP recommended settings.
#[must_use]
pub fn create_password_hasher() -> Argon2PasswordHasher {
    Argon2PasswordHasher::new()
}

/// Create a default JWT generator instance.
///
/// Returns a [`JwtGeneratorImpl`] with default settings (15 min access, 7 day refresh).
///
/// # Warning
/// This uses a default secret key. In production, use [`create_jwt_generator_with_secret`].
#[must_use]
pub fn create_jwt_generator() -> JwtGeneratorImpl {
    JwtGeneratorImpl::default()
}

/// Create a JWT generator with a custom secret key.
///
/// # Arguments
/// * `secret` - The secret key for signing and verifying tokens
///
/// # Returns
/// A [`JwtGeneratorImpl`] configured with the provided secret
///
/// # Example
/// ```rust
/// use zercle_rust_template::internal::infrastructure::security::create_jwt_generator_with_secret;
///
/// let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
/// let generator = create_jwt_generator_with_secret(secret.as_bytes());
/// ```
#[must_use]
pub fn create_jwt_generator_with_secret(secret: &[u8]) -> JwtGeneratorImpl {
    JwtGeneratorImpl::new(secret)
}

/// Create a JWT generator with custom token durations.
///
/// # Arguments
/// * `secret` - The secret key for signing and verifying tokens
/// * `access_token_minutes` - Access token validity in minutes
/// * `refresh_token_days` - Refresh token validity in days
///
/// # Returns
/// A [`JwtGeneratorImpl`] with custom durations
#[must_use]
pub fn create_jwt_generator_with_durations(
    secret: &[u8],
    access_token_minutes: i64,
    refresh_token_days: i64,
) -> JwtGeneratorImpl {
    JwtGeneratorImpl::with_durations(secret, access_token_minutes, refresh_token_days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::domain::user::traits::{PasswordHasher, JwtGenerator};
    use uuid::Uuid;

    #[test]
    fn test_create_password_hasher() {
        let hasher = create_password_hasher();
        let password = "test_password";
        let hash = hasher.hash_password(password).expect("Should hash password");
        let verified = hasher.verify_password(password, &hash).expect("Should verify password");
        assert!(verified);
    }

    #[test]
    fn test_create_jwt_generator() {
        let generator = create_jwt_generator();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let access_token = generator
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let (user_id_result, email_result) = generator
            .validate_access_token(&access_token)
            .expect("Should validate access token");

        assert_eq!(user_id, user_id_result);
        assert_eq!(email, email_result);
    }

    #[test]
    fn test_create_jwt_generator_with_secret() {
        let secret = b"test-secret-key-256-bits-long!";
        let generator = create_jwt_generator_with_secret(secret);
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let access_token = generator
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let (user_id_result, email_result) = generator
            .validate_access_token(&access_token)
            .expect("Should validate access token");

        assert_eq!(user_id, user_id_result);
        assert_eq!(email, email_result);
    }

    #[test]
    fn test_create_jwt_generator_with_durations() {
        let secret = b"test-secret-key-256-bits-long!";
        let generator = create_jwt_generator_with_durations(secret, 60, 14);
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let access_token = generator
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let (user_id_result, email_result) = generator
            .validate_access_token(&access_token)
            .expect("Should validate access token");

        assert_eq!(user_id, user_id_result);
        assert_eq!(email, email_result);
    }
}
