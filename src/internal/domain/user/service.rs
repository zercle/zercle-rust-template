use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::task::traits::TaskRepository;
use crate::internal::domain::user::dto::{
    LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterRequest,
    UpdateProfileRequest, UserResponse,
};
use crate::internal::domain::user::entity::{RefreshToken, User};
use crate::internal::domain::user::traits::{
    JwtGenerator, PasswordHasher, RefreshTokenRepository, UserRepository, UserService,
};

/// User service implementation with all business logic
pub struct UserServiceImpl {
    user_repo: Arc<dyn UserRepository>,
    refresh_repo: Arc<dyn RefreshTokenRepository>,
    task_repo: Arc<dyn TaskRepository>,
    hasher: Arc<dyn PasswordHasher>,
    jwt: Arc<dyn JwtGenerator>,
}

impl UserServiceImpl {
    /// Create a new UserServiceImpl
    #[allow(dead_code)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        refresh_repo: Arc<dyn RefreshTokenRepository>,
        task_repo: Arc<dyn TaskRepository>,
        hasher: Arc<dyn PasswordHasher>,
        jwt: Arc<dyn JwtGenerator>,
    ) -> Self {
        Self {
            user_repo,
            refresh_repo,
            task_repo,
            hasher,
            jwt,
        }
    }

    /// Convert User entity to UserResponse DTO
    fn user_to_response(user: &User) -> UserResponse {
        UserResponse {
            id: user.id,
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }

    /// Generate login response with tokens and user data
    fn generate_login_response(
        &self,
        user: User,
        access_token: String,
        refresh_token: String,
        expires_in: u64,
    ) -> Result<LoginResponse, DomainError> {
        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            user: Self::user_to_response(&user),
        })
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    /// Register a new user
    async fn register(&self, req: RegisterRequest) -> Result<LoginResponse, DomainError> {
        // Check if email already exists
        if self.user_repo.exists_by_email(&req.email).await? {
            return Err(DomainError::EmailAlreadyExists);
        }

        // Hash password
        let password_hash = self.hasher.hash_password(&req.password)?;

        // Create new user entity
        let user = User::new(
            Uuid::new_v4(),
            req.email.clone(),
            password_hash,
            Some(req.full_name.clone()),
        );

        // Save to repository
        self.user_repo.create(&user).await?;

        // Generate access and refresh tokens
        let access_token = self.jwt.generate_access_token(user.id, &user.email)?;
        let (refresh_token, expires_at) = self.jwt.generate_refresh_token(user.id)?;

        // Save refresh token
        let refresh_token_entity = RefreshToken::new(Uuid::new_v4(), user.id, refresh_token.clone(), expires_at);
        self.refresh_repo.create(&refresh_token_entity).await?;

        // Return login response
        self.generate_login_response(user, access_token, refresh_token, 3600)
    }

    /// Authenticate user and return tokens
    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, DomainError> {
        // Find user by email
        let user = match self.user_repo.get_by_email(&req.email).await {
            Ok(user) => user,
            Err(DomainError::UserNotFound) => return Err(DomainError::InvalidCredentials),
            Err(e) => return Err(e),
        };

        // Verify password
        if !self.hasher.verify_password(&req.password, &user.password_hash)? {
            return Err(DomainError::InvalidCredentials);
        }

        // Generate new access and refresh tokens
        let access_token = self.jwt.generate_access_token(user.id, &user.email)?;
        let (refresh_token, expires_at) = self.jwt.generate_refresh_token(user.id)?;

        // Save refresh token
        let refresh_token_entity = RefreshToken::new(Uuid::new_v4(), user.id, refresh_token.clone(), expires_at);
        self.refresh_repo.create(&refresh_token_entity).await?;

        // Return login response
        self.generate_login_response(user, access_token, refresh_token, 3600)
    }

    /// Refresh access token
    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, DomainError> {
        // Get refresh token from repository
        let stored_token = match self.refresh_repo.get_by_token(&req.refresh_token).await {
            Ok(token) => token,
            Err(DomainError::UserNotFound) => return Err(DomainError::TokenInvalid),
            Err(e) => return Err(e),
        };

        // Check if token is expired
        if stored_token.expires_at < Utc::now() {
            return Err(DomainError::TokenExpired);
        }

        // Validate token using JwtGenerator
        let (user_id, email) = self.jwt.validate_access_token(&req.refresh_token)?;

        // Generate new access token
        let access_token = self.jwt.generate_access_token(user_id, &email)?;

        // Generate new refresh token (rotation)
        let (new_refresh_token, new_expires_at) = self.jwt.generate_refresh_token(user_id)?;

        // Delete old refresh token
        self.refresh_repo.delete_by_token(&req.refresh_token).await?;

        // Save new refresh token
        let new_token_entity = RefreshToken::new(Uuid::new_v4(), user_id, new_refresh_token.clone(), new_expires_at);
        self.refresh_repo.create(&new_token_entity).await?;

        // Return refresh response
        Ok(RefreshResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }

    /// Logout user by invalidating refresh token
    async fn logout(&self, _user_id: Uuid, refresh_token: String) -> Result<(), DomainError> {
        self.refresh_repo.delete_by_token(&refresh_token).await
    }

    /// Get user profile
    async fn get_profile(&self, user_id: Uuid) -> Result<UserResponse, DomainError> {
        let user = self.user_repo.get_by_id(user_id).await?;
        Ok(Self::user_to_response(&user))
    }

    /// Update user profile
    async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<UserResponse, DomainError> {
        // Get user by ID
        let mut user = self.user_repo.get_by_id(user_id).await?;

        // Update fields if provided
        if let Some(email) = &req.email {
            // If email changed, check it's not taken
            if email != &user.email {
                if self.user_repo.exists_by_email(email).await? {
                    return Err(DomainError::EmailAlreadyExists);
                }
                user.email = email.clone();
            }
        }

        if let Some(full_name) = &req.full_name {
            user.full_name = Some(full_name.clone());
        }

        // Update timestamp
        user.updated_at = Utc::now();

        // Save and return
        self.user_repo.update(&user).await?;
        Ok(Self::user_to_response(&user))
    }

    /// Delete user account
    async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError> {
        // Delete all refresh tokens for user
        let _ = self.refresh_repo.delete_by_user_id(user_id).await;

        // Delete all tasks for user
        let _ = self.task_repo.delete_by_user(user_id).await;

        // Delete user
        self.user_repo.delete(user_id).await
    }
}
