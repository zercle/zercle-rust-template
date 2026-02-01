//! Unit tests for security components
//!
//! Tests password hashing with Argon2id and JWT token generation/validation.
//! These tests verify the core security infrastructure of the application.

use uuid::Uuid;

use zercle_rust_template::internal::domain::error::DomainError;
use zercle_rust_template::internal::infrastructure::security::{
    jwt::JwtGeneratorImpl,
    password::Argon2PasswordHasher,
};

/// ================================
/// Password Hashing Tests
/// ================================

#[test]
fn test_password_hashing_returns_valid_hash() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "SecureP@ss123!";

    // Act
    let hash = hasher.hash_password(password).expect("Should hash password");

    // Assert
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should start with argon2id marker"
    );
    assert!(
        hash.contains("$v=19$"),
        "Hash should contain version marker"
    );
    assert!(
        hash.contains('$'),
        "Hash should contain salt separator"
    );
}

#[test]
fn test_verify_password_success() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "MySecurePassword123!";
    let hash = hasher.hash_password(password).expect("Should hash password");

    // Act
    let result = hasher.verify_password(password, &hash).expect("Should verify password");

    // Assert
    assert!(result, "Correct password should verify successfully");
}

#[test]
fn test_verify_password_failure() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "MySecurePassword123!";
    let wrong_password = "WrongPassword456!";
    let hash = hasher.hash_password(password).expect("Should hash password");

    // Act
    let result = hasher
        .verify_password(wrong_password, &hash)
        .expect("Should verify password");

    // Assert
    assert!(!result, "Wrong password should not verify");
}

#[test]
fn test_different_salts_produce_different_hashes() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "SamePassword123!";

    // Act
    let hash1 = hasher
        .hash_password(password)
        .expect("Should hash password");
    let hash2 = hasher
        .hash_password(password)
        .expect("Should hash password");

    // Assert
    assert_ne!(
        hash1, hash2,
        "Same password should produce different hashes due to random salt"
    );
}

#[test]
fn test_hash_password_empty_password() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();

    // Act
    let result = hasher.hash_password("");

    // Assert
    assert!(result.is_ok(), "Should handle empty password");
}

#[test]
fn test_verify_password_empty_password() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let hash = hasher.hash_password("").expect("Should hash password");

    // Act
    let result = hasher.verify_password("", &hash).expect("Should verify");

    // Assert
    assert!(result, "Empty password should verify against empty hash");
}

#[test]
fn test_verify_password_invalid_hash_format() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();

    // Act
    let result = hasher.verify_password("password", "invalid_hash_format");

    // Assert
    assert!(result.is_ok(), "Invalid hash should return false, not error");
    assert!(!result.unwrap(), "Invalid hash should not verify");
}

#[test]
fn test_password_hash_unicode_characters() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "P@ssw0rd!@#$%^&*()";

    // Act
    let hash = hasher.hash_password(password).expect("Should hash password");
    let result = hasher.verify_password(password, &hash).expect("Should verify");

    // Assert
    assert!(result, "Password with special characters should verify");
}

#[test]
fn test_password_hash_long_password() {
    // Arrange
    let hasher = Argon2PasswordHasher::new();
    let password = "a".repeat(1000); // Very long password

    // Act
    let hash = hasher.hash_password(&password).expect("Should hash password");
    let result = hasher.verify_password(&password, &hash).expect("Should verify");

    // Assert
    assert!(result, "Long password should hash and verify correctly");
}

/// ================================
/// JWT Token Tests
/// ================================

#[test]
fn test_jwt_generate_and_validate_access_token() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
    let user_id = Uuid::new_v4();
    let email = "test@example.com";

    // Act
    let token = generator
        .generate_access_token(user_id, email)
        .expect("Should generate access token");
    let (validated_user_id, validated_email) = generator
        .validate_access_token(&token)
        .expect("Should validate access token");

    // Assert
    assert!(!token.is_empty(), "Token should not be empty");
    assert_eq!(user_id, validated_user_id);
    assert_eq!(email, validated_email);
}

#[test]
fn test_jwt_generate_refresh_token() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
    let user_id = Uuid::new_v4();

    // Act
    let (token, expiration) = generator
        .generate_refresh_token(user_id)
        .expect("Should generate refresh token");

    // Assert
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(
        expiration > chrono::Utc::now(),
        "Expiration should be in the future"
    );
}

#[test]
fn test_jwt_invalid_token_returns_error() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");

    // Act
    let result = generator.validate_access_token("invalid.token.here");

    // Assert
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
}

#[test]
fn test_jwt_expired_token_returns_error() {
    // Arrange - Use negative duration to create already expired token
    let generator = JwtGeneratorImpl::with_durations(b"test-secret", -1, 7);
    let user_id = Uuid::new_v4();
    let email = "test@example.com";

    // Act
    let token = generator
        .generate_access_token(user_id, email)
        .expect("Should generate access token");
    let result = generator.validate_access_token(&token);

    // Assert
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DomainError::TokenExpired));
}

#[test]
fn test_jwt_wrong_secret_returns_invalid_error() {
    // Arrange
    let generator1 = JwtGeneratorImpl::new(b"secret-key-1");
    let generator2 = JwtGeneratorImpl::new(b"secret-key-2");
    let user_id = Uuid::new_v4();
    let email = "test@example.com";

    // Act - Generate token with one secret, validate with another
    let token = generator1
        .generate_access_token(user_id, email)
        .expect("Should generate access token");
    let result = generator2.validate_access_token(&token);

    // Assert
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
}

#[test]
fn test_jwt_malformed_token_returns_error() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");

    // Act
    let result = generator.validate_access_token("not-a-jwt");

    // Assert
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
}

#[test]
fn test_jwt_empty_token_returns_error() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");

    // Act
    let result = generator.validate_access_token("");

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_jwt_token_contains_correct_claims() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
    let user_id = Uuid::new_v4();
    let email = "claims_test@example.com";

    // Act
    let token = generator
        .generate_access_token(user_id, email)
        .expect("Should generate access token");
    let (validated_user_id, validated_email) = generator
        .validate_access_token(&token)
        .expect("Should validate access token");

    // Assert - Claims should be preserved
    assert_eq!(user_id, validated_user_id);
    assert_eq!(email, validated_email);
}

#[test]
fn test_jwt_different_user_ids_produce_different_tokens() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
    let email = "same@example.com";
    let user_id1 = Uuid::new_v4();
    let user_id2 = Uuid::new_v4();

    // Act
    let token1 = generator
        .generate_access_token(user_id1, email)
        .expect("Should generate token");
    let token2 = generator
        .generate_access_token(user_id2, email)
        .expect("Should generate token");

    // Assert - JWTs should be different for different users
    assert_ne!(token1, token2, "Different users should get different tokens");
}

#[test]
fn test_jwt_clone_works_correctly() {
    // Arrange
    let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
    let user_id = Uuid::new_v4();
    let email = "clone_test@example.com";

    // Act
    let token = generator
        .generate_access_token(user_id, email)
        .expect("Should generate token");
    let cloned_token = generator
        .generate_access_token(user_id, email)
        .expect("Should generate token from cloned generator");

    // Assert - Both should work
    let result1 = generator.validate_access_token(&token);
    let result2 = generator.validate_access_token(&cloned_token);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}
