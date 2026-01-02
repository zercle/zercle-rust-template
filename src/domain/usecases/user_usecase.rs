//! User use cases implementation
//!
//! This module contains the business logic for user operations including
//! registration, login, profile management, and user listing.

use crate::config::Settings;
use crate::domain::entities::{
    CreateUser, CreateUserRequest, LoginRequest, UpdateUserRequest, User,
};
use crate::domain::repositories::UserRepository;
use anyhow::{Context, Result};
use argon2::password_hash::Error as PasswordHashError;
use argon2::{
    password_hash::rand_core::OsRng, Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Custom error types for user use case operations
#[derive(thiserror::Error, Debug)]
pub enum UserUsecaseError {
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("User already exists with email: {0}")]
    UserAlreadyExists(String),

    #[error("User not found with id: {0}")]
    UserNotFound(Uuid),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<anyhow::Error> for UserUsecaseError {
    fn from(e: anyhow::Error) -> Self {
        UserUsecaseError::DatabaseError(e.to_string())
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // User ID
    pub email: String, // User email
    pub exp: i64,      // Expiration time
    pub iat: i64,      // Issued at
}

/// Response structure for login/registration
#[derive(Debug)]
pub struct AuthResponse {
    pub user: User,
    pub token: String,
}

/// User use case trait
#[async_trait]
pub trait UserUsecase: Send + Sync {
    /// Register a new user
    async fn register(&self, req: CreateUserRequest) -> Result<AuthResponse, UserUsecaseError>;

    /// Login a user
    async fn login(&self, req: LoginRequest) -> Result<AuthResponse, UserUsecaseError>;

    /// Get user profile
    async fn get_profile(&self, user_id: Uuid) -> Result<User, UserUsecaseError>;

    /// Update user profile
    async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateUserRequest,
    ) -> Result<User, UserUsecaseError>;

    /// Delete user account
    async fn delete_account(&self, user_id: Uuid) -> Result<(), UserUsecaseError>;

    /// List users with pagination
    async fn list_users(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<User>, i64), UserUsecaseError>;
}

/// User use case implementation
pub struct UserUsecaseImpl {
    user_repo: Arc<dyn UserRepository>,
    jwt_secret: String,
    jwt_expiration_hours: i64,
    argon2: Argon2<'static>,
}

impl UserUsecaseImpl {
    /// Create a new UserUsecaseImpl
    ///
    /// # Arguments
    /// * `user_repo` - User repository implementation
    /// * `settings` - Application settings for JWT configuration
    ///
    /// # Returns
    /// A new UserUsecaseImpl instance
    pub fn new(user_repo: Arc<dyn UserRepository>, settings: &Settings) -> Self {
        Self {
            user_repo,
            jwt_secret: settings.jwt.secret.clone(),
            jwt_expiration_hours: settings.jwt.expiration_hours as i64,
            argon2: Argon2::default(),
        }
    }

    /// Hash a password using argon2id
    ///
    /// # Arguments
    /// * `password` - Plain text password
    ///
    /// # Returns
    /// Result containing the hashed password or an error
    fn hash_password(&self, password: &str) -> Result<String, UserUsecaseError> {
        use argon2::password_hash::SaltString;
        let password_bytes = password.as_bytes();

        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = self
            .argon2
            .hash_password(password_bytes, &salt)
            .map_err(|e| UserUsecaseError::AuthError(format!("Failed to hash password: {}", e)))?;

        Ok(hashed_password.to_string())
    }

    /// Verify a password against a hash
    ///
    /// # Arguments
    /// * `password` - Plain text password
    /// * `hash` - Hashed password
    ///
    /// # Returns
    /// Result indicating if the password matches
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, UserUsecaseError> {
        let password_bytes = password.as_bytes();

        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            UserUsecaseError::AuthError(format!("Failed to parse password hash: {}", e))
        })?;

        let result = self.argon2.verify_password(password_bytes, &parsed_hash);

        match result {
            Ok(()) => Ok(true),
            Err(PasswordHashError::Password) => Ok(false),
            Err(e) => Err(UserUsecaseError::AuthError(format!(
                "Password verification failed: {}",
                e
            ))),
        }
    }

    /// Generate a JWT token
    ///
    /// # Arguments
    /// * `user_id` - User's UUID
    /// * `email` - User's email
    ///
    /// # Returns
    /// Result containing the JWT token or an error
    fn generate_token(&self, user_id: Uuid, email: &str) -> Result<String, UserUsecaseError> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.jwt_expiration_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .context("Failed to encode JWT token")?;

        Ok(token)
    }
}

#[async_trait]
impl UserUsecase for UserUsecaseImpl {
    /// Register a new user
    async fn register(&self, req: CreateUserRequest) -> Result<AuthResponse, UserUsecaseError> {
        // Validate the request
        req.validate_request()
            .map_err(|e| UserUsecaseError::ValidationError(e.to_string()))?;

        // Check if user already exists
        let existing = self
            .user_repo
            .find_by_email(&req.email)
            .await
            .context("Failed to check for existing user")?;

        if existing.is_some() {
            return Err(UserUsecaseError::UserAlreadyExists(req.email.clone()));
        }

        // Hash the password
        let password_hash = self.hash_password(&req.password)?;

        // Create the user
        let create_user = CreateUser::new(
            req.email.clone(),
            password_hash,
            req.full_name.clone(),
            req.phone.clone(),
        );

        let user = self
            .user_repo
            .create(&create_user)
            .await
            .context("Failed to create user")?;

        // Generate JWT token
        let token = self.generate_token(user.id, &user.email)?;

        Ok(AuthResponse { user, token })
    }

    /// Login a user
    async fn login(&self, req: LoginRequest) -> Result<AuthResponse, UserUsecaseError> {
        // Validate the request
        req.validate_request()
            .map_err(|e| UserUsecaseError::ValidationError(e.to_string()))?;

        // Find user by email
        let user = self
            .user_repo
            .find_by_email(&req.email)
            .await
            .context("Failed to find user by email")?
            .ok_or(UserUsecaseError::InvalidCredentials(
                "Invalid email or password".to_string(),
            ))?;

        // Verify password
        let password_valid = self.verify_password(&req.password, &user.password_hash)?;

        if !password_valid {
            return Err(UserUsecaseError::InvalidCredentials(
                "Invalid email or password".to_string(),
            ));
        }

        // Generate JWT token
        let token = self.generate_token(user.id, &user.email)?;

        Ok(AuthResponse { user, token })
    }

    /// Get user profile
    async fn get_profile(&self, user_id: Uuid) -> Result<User, UserUsecaseError> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .context("Failed to find user by id")?
            .ok_or(UserUsecaseError::UserNotFound(user_id))?;

        Ok(user)
    }

    /// Update user profile
    async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateUserRequest,
    ) -> Result<User, UserUsecaseError> {
        // Validate the request
        req.validate_request()
            .map_err(|e| UserUsecaseError::ValidationError(e.to_string()))?;

        // Check if there are any updates
        if !req.has_updates() {
            // Return existing user if no updates
            return self.get_profile(user_id).await;
        }

        // Find existing user
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .context("Failed to find user by id")?
            .ok_or(UserUsecaseError::UserNotFound(user_id))?;

        // Apply updates
        if let Some(full_name) = req.full_name {
            if full_name.len() < 2 {
                return Err(UserUsecaseError::ValidationError(
                    "Full name must be at least 2 characters".to_string(),
                ));
            }
            user.full_name = Some(full_name);
        }

        if let Some(phone) = req.phone {
            user.phone = Some(phone);
        }

        // Update in repository
        let updated_user = self
            .user_repo
            .update(&user)
            .await
            .context("Failed to update user")?;

        Ok(updated_user)
    }

    /// Delete user account
    async fn delete_account(&self, user_id: Uuid) -> Result<(), UserUsecaseError> {
        // Verify user exists first
        let _user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .context("Failed to find user by id")?
            .ok_or(UserUsecaseError::UserNotFound(user_id))?;

        self.user_repo
            .delete(user_id)
            .await
            .context("Failed to delete user")?;

        Ok(())
    }

    /// List users with pagination
    async fn list_users(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<User>, i64), UserUsecaseError> {
        let (users, total) = self
            .user_repo
            .list(limit, offset)
            .await
            .context("Failed to list users")?;

        Ok((users, total))
    }
}

/// Login response structure
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::UserRepository;
    use async_trait::async_trait;
    use uuid::Uuid;

    // Mock user repository for testing
    struct MockUserRepository {
        users: std::sync::Mutex<Vec<User>>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create(&self, user: &CreateUser) -> Result<User> {
            let mut users = self.users.lock().unwrap();
            let id = Uuid::new_v4();
            let now = Utc::now();
            let new_user = User {
                id,
                email: user.email.clone(),
                password_hash: user.password_hash.clone(),
                full_name: user.full_name.clone(),
                phone: user.phone.clone(),
                created_at: now,
                updated_at: now,
            };
            users.push(new_user.clone());
            Ok(new_user)
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.id == id).cloned())
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.email == email).cloned())
        }

        async fn update(&self, user: &User) -> Result<User> {
            let mut users = self.users.lock().unwrap();
            if let Some(idx) = users.iter().position(|u| u.id == user.id) {
                users[idx] = user.clone();
                Ok(users[idx].clone())
            } else {
                Err(anyhow::anyhow!("User not found"))
            }
        }

        async fn delete(&self, id: Uuid) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            users.retain(|u| u.id != id);
            Ok(())
        }

        async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<User>, i64)> {
            let users = self.users.lock().unwrap();
            let total = users.len() as i64;
            let users: Vec<User> = users
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok((users, total))
        }

        async fn count(&self) -> Result<i64> {
            let users = self.users.lock().unwrap();
            Ok(users.len() as i64)
        }
    }

    #[tokio::test]
    async fn test_register_success() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository {
            users: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = UserUsecaseImpl::new(mock_repo, &settings);

        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: Some("+1234567890".to_string()),
        };

        let result = usecase.register(req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.user.email, "test@example.com");
        assert!(!response.token.is_empty());
    }

    #[tokio::test]
    async fn test_login_success() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository {
            users: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        // First register
        let register_req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: Some("+1234567890".to_string()),
        };
        usecase.register(register_req).await.unwrap();

        // Then login
        let login_req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
        };

        let result = usecase.login(login_req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.user.email, "test@example.com");
        assert!(!response.token.is_empty());
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository {
            users: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        // First register
        let register_req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: None,
        };
        usecase.register(register_req).await.unwrap();

        // Then login with wrong password
        let login_req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "WrongPassword123!".to_string(),
        };

        let result = usecase.login(login_req).await;
        assert!(result.is_err());
    }
}
