use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use validator::{Validate, ValidationErrors};

/// Errors that can occur during user entity validation
#[derive(Debug, Error)]
pub enum UserValidationError {
    #[error("Email validation failed: {0}")]
    EmailError(String),

    #[error("Password validation failed: {0}")]
    PasswordError(String),

    #[error("Phone validation failed: {0}")]
    PhoneError(String),

    #[error("Full name validation failed: {0}")]
    FullNameError(String),
}

/// User entity representing a user in the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new user entity
    pub fn new(
        id: Uuid,
        email: String,
        password_hash: String,
        full_name: Option<String>,
        phone: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            email,
            password_hash,
            full_name,
            phone,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Request to create a new user
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    /// User's email address (must be valid format)
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// User's password (min 8 chars)
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    /// Optional full name
    #[validate(length(max = 255, message = "Full name must not exceed 255 characters"))]
    pub full_name: Option<String>,

    /// Optional phone number
    #[validate(length(max = 20, message = "Phone number must not exceed 20 characters"))]
    pub phone: Option<String>,
}

impl CreateUserRequest {
    /// Validate the create user request
    pub fn validate_request(&self) -> Result<(), UserValidationError> {
        // Run validator crate's built-in validations
        self.validate().map_err(|errors: ValidationErrors| {
            // Get the first error
            if let Some((field, errors)) = errors.field_errors().into_iter().next() {
                if let Some(error) = errors.first() {
                    let message = error.message.as_deref().unwrap_or("Validation failed");
                    let field_str: &str = &*field;
                    return match field_str {
                        "email" => UserValidationError::EmailError(message.to_string()),
                        "password" => UserValidationError::PasswordError(message.to_string()),
                        "full_name" => UserValidationError::FullNameError(message.to_string()),
                        "phone" => UserValidationError::PhoneError(message.to_string()),
                        _ => UserValidationError::PasswordError(message.to_string()),
                    };
                }
            }
            UserValidationError::PasswordError("Validation failed".to_string())
        })?;

        // Custom password complexity validation
        let password = &self.password;
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_upper {
            return Err(UserValidationError::PasswordError(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        if !has_lower {
            return Err(UserValidationError::PasswordError(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        if !has_digit {
            return Err(UserValidationError::PasswordError(
                "Password must contain at least one number".to_string(),
            ));
        }

        if !has_special {
            return Err(UserValidationError::PasswordError(
                "Password must contain at least one special character".to_string(),
            ));
        }

        Ok(())
    }
}

/// Request to update an existing user
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUserRequest {
    /// Optional full name to update
    #[validate(length(max = 255, message = "Full name must not exceed 255 characters"))]
    pub full_name: Option<String>,

    /// Optional phone number to update
    #[validate(length(max = 20, message = "Phone number must not exceed 20 characters"))]
    pub phone: Option<String>,
}

impl UpdateUserRequest {
    /// Validate the update user request
    pub fn validate_request(&self) -> Result<(), UserValidationError> {
        self.validate().map_err(|errors: ValidationErrors| {
            if let Some((field, errors)) = errors.field_errors().into_iter().next() {
                if let Some(error) = errors.first() {
                    let message = error.message.as_deref().unwrap_or("Validation failed");
                    let field_str: &str = &*field;
                    return match field_str {
                        "full_name" => UserValidationError::FullNameError(message.to_string()),
                        "phone" => UserValidationError::PhoneError(message.to_string()),
                        _ => UserValidationError::FullNameError(message.to_string()),
                    };
                }
            }
            UserValidationError::FullNameError("Validation failed".to_string())
        })?;

        Ok(())
    }

    /// Check if the update request has any fields to update
    pub fn has_updates(&self) -> bool {
        self.full_name.is_some() || self.phone.is_some()
    }
}

/// Request for user login
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    /// User's email address
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// User's password
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

impl LoginRequest {
    /// Validate the login request
    pub fn validate_request(&self) -> Result<(), UserValidationError> {
        self.validate().map_err(|errors: ValidationErrors| {
            if let Some((field, errors)) = errors.field_errors().into_iter().next() {
                if let Some(error) = errors.first() {
                    let message = error.message.as_deref().unwrap_or("Validation failed");
                    let field_str: &str = &*field;
                    return match field_str {
                        "email" => UserValidationError::EmailError(message.to_string()),
                        "password" => UserValidationError::PasswordError(message.to_string()),
                        _ => UserValidationError::EmailError(message.to_string()),
                    };
                }
            }
            UserValidationError::EmailError("Validation failed".to_string())
        })?;

        Ok(())
    }
}

/// Data structure for creating a new user (repository layer)
#[derive(Debug, Clone)]
pub struct CreateUser {
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
}

impl CreateUser {
    /// Create a new CreateUser instance
    pub fn new(
        email: String,
        password_hash: String,
        full_name: Option<String>,
        phone: Option<String>,
    ) -> Self {
        Self {
            email,
            password_hash,
            full_name,
            phone,
        }
    }
}

/// Data structure for updating a user (repository layer)
#[derive(Debug, Default, Clone)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
    pub phone: Option<String>,
}

impl UpdateUser {
    /// Create a new UpdateUser instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the email
    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    /// Set the password hash
    pub fn with_password_hash(mut self, password_hash: String) -> Self {
        self.password_hash = Some(password_hash);
        self
    }

    /// Set the full name
    pub fn with_full_name(mut self, full_name: Option<String>) -> Self {
        self.full_name = full_name;
        self
    }

    /// Set the phone
    pub fn with_phone(mut self, phone: Option<String>) -> Self {
        self.phone = phone;
        self
    }

    /// Check if there are any updates
    pub fn has_updates(&self) -> bool {
        self.email.is_some()
            || self.password_hash.is_some()
            || self.full_name.is_some()
            || self.phone.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new() {
        let id = Uuid::new_v4();
        let user = User::new(
            id,
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("John Doe".to_string()),
            Some("+1234567890".to_string()),
        );

        assert_eq!(user.id, id);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
        assert_eq!(user.full_name, Some("John Doe".to_string()));
        assert_eq!(user.phone, Some("+1234567890".to_string()));
    }

    #[test]
    fn test_create_user_request_valid() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("John Doe".to_string()),
            phone: Some("+1234567890".to_string()),
        };

        assert!(request.validate_request().is_ok());
    }

    #[test]
    fn test_create_user_request_invalid_email() {
        let request = CreateUserRequest {
            email: "invalid-email".to_string(),
            password: "Password123!".to_string(),
            full_name: None,
            phone: None,
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_create_user_request_invalid_password_length() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Short1!".to_string(),
            full_name: None,
            phone: None,
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_create_user_request_invalid_password_complexity() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "password".to_string(),
            full_name: None,
            phone: None,
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_update_user_request_valid() {
        let request = UpdateUserRequest {
            full_name: Some("Jane Doe".to_string()),
            phone: Some("+0987654321".to_string()),
        };

        assert!(request.validate_request().is_ok());
        assert!(request.has_updates());
    }

    #[test]
    fn test_update_user_request_empty() {
        let request = UpdateUserRequest {
            full_name: None,
            phone: None,
        };

        assert!(request.validate_request().is_ok());
        assert!(!request.has_updates());
    }

    #[test]
    fn test_login_request_valid() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert!(request.validate_request().is_ok());
    }

    #[test]
    fn test_login_request_invalid_email() {
        let request = LoginRequest {
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_login_request_empty_password() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };

        assert!(request.validate_request().is_err());
    }
}
