use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request DTO for user registration
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 2, message = "Full name must be at least 2 characters"))]
    pub full_name: String,
}

/// Request DTO for user login
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Request DTO for token refresh
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

/// Request DTO for updating user profile
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,

    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
}

/// Response DTO for user data
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response DTO for login operation
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// Response DTO for token refresh operation
#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
