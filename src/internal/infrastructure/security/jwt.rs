use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use jsonwebtoken::errors::ErrorKind;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::user::traits::JwtGenerator;

/// JWT claims structure for access and refresh tokens.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    #[serde(rename = "sub")]
    pub subject: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(rename = "exp")]
    pub expiration: i64,

    #[serde(rename = "iat")]
    pub issued_at: i64,

    #[serde(rename = "jti")]
    pub jwt_id: String,
}

impl Claims {
    pub fn new_access_claims(user_id: Uuid, email: &str, duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            subject: user_id.to_string(),
            email: Some(email.to_string()),
            expiration: (now + duration).timestamp(),
            issued_at: now.timestamp(),
            jwt_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn new_refresh_claims(user_id: Uuid, duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            subject: user_id.to_string(),
            email: None,
            expiration: (now + duration).timestamp(),
            issued_at: now.timestamp(),
            jwt_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn user_id(&self) -> Result<Uuid, DomainError> {
        Uuid::parse_str(&self.subject).map_err(|_| DomainError::TokenInvalid)
    }

    pub fn get_email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

/// JWT token generator and validator implementation.
pub struct JwtGeneratorImpl {
    /// Secret key bytes (stored for cloning)
    secret: Vec<u8>,
    /// Token expiration duration for access tokens (15 minutes)
    access_token_duration: Duration,
    /// Token expiration duration for refresh tokens (7 days)
    refresh_token_duration: Duration,
}

impl Clone for JwtGeneratorImpl {
    fn clone(&self) -> Self {
        Self {
            secret: self.secret.clone(),
            access_token_duration: self.access_token_duration,
            refresh_token_duration: self.refresh_token_duration,
        }
    }
}

impl JwtGeneratorImpl {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            access_token_duration: Duration::minutes(15),
            refresh_token_duration: Duration::days(7),
        }
    }

    pub fn with_durations(
        secret: &[u8],
        access_token_minutes: i64,
        refresh_token_days: i64,
    ) -> Self {
        Self {
            secret: secret.to_vec(),
            access_token_duration: Duration::minutes(access_token_minutes),
            refresh_token_duration: Duration::days(refresh_token_days),
        }
    }

    /// Get the encoding key.
    fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(&self.secret)
    }

    /// Get the decoding key.
    fn decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(&self.secret)
    }
}

impl Default for JwtGeneratorImpl {
    fn default() -> Self {
        let secret = b"your-256-bit-secret-key-for-jwt-signing-change-in-production";
        Self::new(secret)
    }
}

impl JwtGenerator for JwtGeneratorImpl {
    fn generate_access_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError> {
        let claims = Claims::new_access_claims(user_id, email, self.access_token_duration);

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key()).map_err(|e| {
            tracing::error!(error = %e, "Failed to generate access token");
            DomainError::Internal
        })
    }

    fn generate_refresh_token(&self, user_id: Uuid) -> Result<(String, DateTime<Utc>), DomainError> {
        let claims = Claims::new_refresh_claims(user_id, self.refresh_token_duration);
        let expiration = DateTime::from_timestamp(claims.expiration, 0)
            .ok_or_else(|| {
                tracing::error!("Failed to create expiration timestamp");
                DomainError::Internal
            })?;

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key())
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate refresh token");
                DomainError::Internal
            })?;

        Ok((token, expiration))
    }

    fn validate_access_token(&self, token: &str) -> Result<(Uuid, String), DomainError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0; // Strict time validation

        let token_data = decode::<Claims>(token, &self.decoding_key(), &validation).map_err(|e| {
            tracing::debug!(error = %e, "Token validation failed");
            match e.kind() {
                ErrorKind::ExpiredSignature => DomainError::TokenExpired,
                _ => DomainError::TokenInvalid,
            }
        })?;

        let user_id = token_data.claims.user_id()?;
        let email = token_data
            .claims
            .email
            .ok_or_else(|| {
                tracing::error!("Email claim missing from access token");
                DomainError::TokenInvalid
            })?;

        Ok((user_id, email))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_access_token() {
        let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = generator
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let (validated_user_id, validated_email) = generator
            .validate_access_token(&token)
            .expect("Should validate access token");

        assert_eq!(user_id, validated_user_id);
        assert_eq!(email, validated_email);
    }

    #[test]
    fn test_generate_refresh_token() {
        let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");
        let user_id = Uuid::new_v4();

        let (token, expiration) = generator
            .generate_refresh_token(user_id)
            .expect("Should generate refresh token");

        assert!(!token.is_empty(), "Token should not be empty");
        assert!(
            expiration > Utc::now(),
            "Expiration should be in the future"
        );
    }

    #[test]
    fn test_invalid_token_returns_error() {
        let generator = JwtGeneratorImpl::new(b"test-secret-key-for-jwt-testing");

        let result = generator.validate_access_token("invalid.token.here");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
    }

    #[test]
    fn test_expired_token_returns_error() {
        // Use a negative duration to create an already expired token
        let generator = JwtGeneratorImpl::with_durations(b"test-secret", -1, 7);
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = generator
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let result = generator.validate_access_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::TokenExpired));
    }

    #[test]
    fn test_wrong_secret_returns_invalid_error() {
        let generator1 = JwtGeneratorImpl::new(b"secret-key-1");
        let generator2 = JwtGeneratorImpl::new(b"secret-key-2");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = generator1
            .generate_access_token(user_id, email)
            .expect("Should generate access token");

        let result = generator2.validate_access_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
    }
}
