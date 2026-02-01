use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::user::dto::{LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterRequest, UpdateProfileRequest, UserResponse};
use crate::internal::domain::user::entity::{RefreshToken, User};

/// Repository trait for user data access operations
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user
    async fn create(&self, user: &User) -> Result<(), DomainError>;

    /// Get user by ID
    async fn get_by_id(&self, id: Uuid) -> Result<User, DomainError>;

    /// Get user by email
    async fn get_by_email(&self, email: &str) -> Result<User, DomainError>;

    /// Update user
    async fn update(&self, user: &User) -> Result<(), DomainError>;

    /// Delete user by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Check if user exists by email
    async fn exists_by_email(&self, email: &str) -> Result<bool, DomainError>;
}

/// Repository trait for refresh token operations
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    /// Create a new refresh token
    async fn create(&self, token: &RefreshToken) -> Result<(), DomainError>;

    /// Get refresh token by token string
    async fn get_by_token(&self, token: &str) -> Result<RefreshToken, DomainError>;

    /// Delete all refresh tokens for a user
    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError>;

    /// Delete refresh token by token string
    async fn delete_by_token(&self, token: &str) -> Result<(), DomainError>;

    /// Delete all expired tokens and return count deleted
    async fn delete_expired(&self) -> Result<u64, DomainError>;
}

/// Service trait for user business logic
#[async_trait]
pub trait UserService: Send + Sync {
    /// Register a new user
    async fn register(&self, req: RegisterRequest) -> Result<LoginResponse, DomainError>;

    /// Authenticate user and return tokens
    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, DomainError>;

    /// Refresh access token
    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, DomainError>;

    /// Logout user by invalidating refresh token
    async fn logout(&self, user_id: Uuid, refresh_token: String) -> Result<(), DomainError>;

    /// Get user profile
    async fn get_profile(&self, user_id: Uuid) -> Result<UserResponse, DomainError>;

    /// Update user profile
    async fn update_profile(&self, user_id: Uuid, req: UpdateProfileRequest) -> Result<UserResponse, DomainError>;

    /// Delete user account
    async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError>;
}

/// Trait for password hashing operations
pub trait PasswordHasher: Send + Sync {
    /// Hash a password
    fn hash_password(&self, password: &str) -> Result<String, DomainError>;

    /// Verify a password against a hash
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, DomainError>;
}

/// Trait for JWT token generation and validation
pub trait JwtGenerator: Send + Sync {
    /// Generate access token for user
    fn generate_access_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError>;

    /// Generate refresh token and return expiration time
    fn generate_refresh_token(&self, user_id: Uuid) -> Result<(String, DateTime<Utc>), DomainError>;

    /// Validate access token and return user ID and email
    fn validate_access_token(&self, token: &str) -> Result<(Uuid, String), DomainError>;
}
