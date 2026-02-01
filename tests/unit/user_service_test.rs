//! Unit tests for UserService
//!
//! Tests the business logic for user registration, login, token refresh,
// profile management, and account deletion.

use std::sync::Arc;
use uuid::Uuid;

use mockall::predicate::*;
use tokio_test;

use zercle_rust_template::internal::domain::{
    error::DomainError,
    task::traits::MockTaskRepository,
    user::{
        dto::{LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterRequest, UpdateProfileRequest},
        entity::{RefreshToken, User},
        service::UserServiceImpl,
        traits::{JwtGenerator, PasswordHasher, RefreshTokenRepository, UserRepository, UserService},
    },
};

/// Mock password hasher for testing
#[derive(Debug, Clone)]
struct MockPasswordHasher;

impl MockPasswordHasher {
    #[allow(dead_code)]
    fn new() -> Self {
        Self
    }
}

impl PasswordHasher for MockPasswordHasher {
    fn hash_password(&self, password: &str) -> Result<String, DomainError> {
        Ok(format!("hashed_{}", password))
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        Ok(hash == format!("hashed_{}", password))
    }
}

/// Mock JWT generator for testing
#[derive(Debug, Clone)]
struct MockJwtGenerator {
    access_token: String,
    refresh_token: String,
    should_fail: bool,
}

impl MockJwtGenerator {
    #[allow(dead_code)]
    fn new(access_token: &str, refresh_token: &str) -> Self {
        Self {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            should_fail: false,
        }
    }

    #[allow(dead_code)]
    fn with_failure(access_token: &str, refresh_token: &str) -> Self {
        Self {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            should_fail: true,
        }
    }
}

impl JwtGenerator for MockJwtGenerator {
    fn generate_access_token(&self, _user_id: Uuid, _email: &str) -> Result<String, DomainError> {
        if self.should_fail {
            Err(DomainError::Internal)
        } else {
            Ok(self.access_token.clone())
        }
    }

    fn generate_refresh_token(&self, _user_id: Uuid) -> Result<(String, chrono::DateTime<chrono::Utc>), DomainError> {
        if self.should_fail {
            Err(DomainError::Internal)
        } else {
            use chrono::{Duration, Utc};
            Ok((self.refresh_token.clone(), Utc::now() + Duration::days(7)))
        }
    }

    fn validate_access_token(&self, token: &str) -> Result<(Uuid, String), DomainError> {
        if self.should_fail {
            Err(DomainError::TokenInvalid)
        } else if token == self.access_token {
            Ok((Uuid::new_v4(), "test@example.com".to_string()))
        } else {
            Err(DomainError::TokenInvalid)
        }
    }
}

/// Helper to create UserService with mocked dependencies
#[allow(dead_code)]
async fn create_mock_service() -> (
    UserServiceImpl,
    MockUserRepository,
    MockRefreshTokenRepository,
    MockTaskRepository,
) {
    let user_repo = MockUserRepository::new();
    let refresh_repo = MockRefreshTokenRepository::new();
    let task_repo = MockTaskRepository::new();
    let hasher: Arc<dyn PasswordHasher> = Arc::new(MockPasswordHasher::new());
    let jwt: Arc<dyn JwtGenerator> = Arc::new(MockJwtGenerator::new("access_token", "refresh_token"));

    let service = UserServiceImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(refresh_repo.clone()),
        Arc::new(task_repo.clone()),
        hasher,
        jwt,
    );

    (service, user_repo, refresh_repo, task_repo)
}

#[tokio::test]
async fn test_register_success() {
    // Arrange
    let (service, mut user_repo, mut refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();

    let register_req = RegisterRequest {
        email: "newuser@example.com".to_string(),
        password: "SecureP@ss123!".to_string(),
        full_name: "New User".to_string(),
    };

    // Mock repository expectations
    user_repo
        .expect_exists_by_email()
        .with(eq("newuser@example.com"))
        .returning(|_| Ok(false));

    user_repo.expect_create().returning(move |user| {
        assert_eq!(user.email, "newuser@example.com");
        Ok(())
    });

    refresh_repo.expect_create().returning(|_| Ok(()));

    // Act
    let result = service.register(register_req).await;

    // Assert
    assert!(result.is_ok(), "Registration should succeed");
    let response = result.unwrap();
    assert_eq!(response.access_token, "access_token");
    assert_eq!(response.refresh_token, "refresh_token");
    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.user.email, "newuser@example.com");
    assert_eq!(response.user.full_name, Some("New User".to_string()));
}

#[tokio::test]
async fn test_register_duplicate_email() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;

    let register_req = RegisterRequest {
        email: "existing@example.com".to_string(),
        password: "SecureP@ss123!".to_string(),
        full_name: "Existing User".to_string(),
    };

    user_repo
        .expect_exists_by_email()
        .with(eq("existing@example.com"))
        .returning(|_| Ok(true));

    // Act
    let result = service.register(register_req).await;

    // Assert
    assert!(result.is_err(), "Registration should fail with duplicate email");
    assert!(matches!(result.unwrap_err(), DomainError::EmailAlreadyExists));
}

#[tokio::test]
async fn test_login_success() {
    // Arrange
    let (service, mut user_repo, mut refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();
    let updated_at = created_at;

    let existing_user = User {
        id: user_id,
        email: "login@example.com".to_string(),
        password_hash: "hashed_SecureP@ss123!".to_string(), // Must match mock hasher
        full_name: Some("Login User".to_string()),
        created_at,
        updated_at,
    };

    let login_req = LoginRequest {
        email: "login@example.com".to_string(),
        password: "SecureP@ss123!".to_string(),
    };

    user_repo
        .expect_get_by_email()
        .with(eq("login@example.com"))
        .returning(move |_| Ok(existing_user.clone()));

    refresh_repo.expect_create().returning(|_| Ok(()));

    // Act
    let result = service.login(login_req).await;

    // Assert
    assert!(result.is_ok(), "Login should succeed with valid credentials");
    let response = result.unwrap();
    assert_eq!(response.access_token, "access_token");
    assert_eq!(response.refresh_token, "refresh_token");
    assert_eq!(response.user.email, "login@example.com");
}

#[tokio::test]
async fn test_login_invalid_credentials_wrong_password() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();
    let updated_at = created_at;

    let existing_user = User {
        id: user_id,
        email: "login@example.com".to_string(),
        password_hash: "hashed_SecureP@ss123!".to_string(),
        full_name: Some("Login User".to_string()),
        created_at,
        updated_at,
    };

    let login_req = LoginRequest {
        email: "login@example.com".to_string(),
        password: "WrongPassword123!".to_string(),
    };

    user_repo
        .expect_get_by_email()
        .with(eq("login@example.com"))
        .returning(move |_| Ok(existing_user.clone()));

    // Act
    let result = service.login(login_req).await;

    // Assert
    assert!(result.is_err(), "Login should fail with wrong password");
    assert!(matches!(result.unwrap_err(), DomainError::InvalidCredentials));
}

#[tokio::test]
async fn test_login_user_not_found() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;

    let login_req = LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "SomePassword123!".to_string(),
    };

    user_repo
        .expect_get_by_email()
        .with(eq("nonexistent@example.com"))
        .returning(|_| Err(DomainError::UserNotFound));

    // Act
    let result = service.login(login_req).await;

    // Assert
    assert!(result.is_err(), "Login should fail when user not found");
    assert!(matches!(result.unwrap_err(), DomainError::InvalidCredentials));
}

#[tokio::test]
async fn test_refresh_token_success() {
    // Arrange
    let (service, mut user_repo, mut refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    let refresh_token_entity = RefreshToken {
        id: Uuid::new_v4(),
        user_id,
        token: "valid_refresh_token".to_string(),
        expires_at,
        created_at: chrono::Utc::now(),
    };

    let refresh_req = RefreshRequest {
        refresh_token: "valid_refresh_token".to_string(),
    };

    refresh_repo
        .expect_get_by_token()
        .with(eq("valid_refresh_token"))
        .returning(move |_| Ok(refresh_token_entity.clone()));

    refresh_repo
        .expect_delete_by_token()
        .with(eq("valid_refresh_token"))
        .returning(|_| Ok(()));

    refresh_repo.expect_create().returning(|_| Ok(()));

    // Act
    let result = service.refresh(refresh_req).await;

    // Assert
    assert!(result.is_ok(), "Token refresh should succeed with valid token");
    let response = result.unwrap();
    assert_eq!(response.access_token, "access_token");
    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.expires_in, 3600);
}

#[tokio::test]
async fn test_refresh_token_expired() {
    // Arrange
    let (service, mut refresh_repo, _user_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let expired_at = chrono::Utc::now() - chrono::Duration::hours(1); // Already expired

    let refresh_token_entity = RefreshToken {
        id: Uuid::new_v4(),
        user_id,
        token: "expired_refresh_token".to_string(),
        expires_at: expired_at,
        created_at: chrono::Utc::now(),
    };

    let refresh_req = RefreshRequest {
        refresh_token: "expired_refresh_token".to_string(),
    };

    refresh_repo
        .expect_get_by_token()
        .with(eq("expired_refresh_token"))
        .returning(move |_| Ok(refresh_token_entity.clone()));

    // Act
    let result = service.refresh(refresh_req).await;

    // Assert
    assert!(result.is_err(), "Token refresh should fail with expired token");
    assert!(matches!(result.unwrap_err(), DomainError::TokenExpired));
}

#[tokio::test]
async fn test_refresh_token_invalid() {
    // Arrange
    let (service, refresh_repo, _user_repo, _task_repo) = create_mock_service().await;

    let refresh_req = RefreshRequest {
        refresh_token: "nonexistent_token".to_string(),
    };

    refresh_repo
        .expect_get_by_token()
        .with(eq("nonexistent_token"))
        .returning(|_| Err(DomainError::UserNotFound));

    // Act
    let result = service.refresh(refresh_req).await;

    // Assert
    assert!(result.is_err(), "Token refresh should fail with invalid token");
    assert!(matches!(result.unwrap_err(), DomainError::TokenInvalid));
}

#[tokio::test]
async fn test_get_profile_success() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();
    let updated_at = created_at;

    let existing_user = User {
        id: user_id,
        email: "profile@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        full_name: Some("Profile User".to_string()),
        created_at,
        updated_at,
    };

    user_repo
        .expect_get_by_id()
        .with(eq(user_id))
        .returning(move |_| Ok(existing_user.clone()));

    // Act
    let result = service.get_profile(user_id).await;

    // Assert
    assert!(result.is_ok(), "Get profile should succeed");
    let response = result.unwrap();
    assert_eq!(response.id, user_id);
    assert_eq!(response.email, "profile@example.com");
    assert_eq!(response.full_name, Some("Profile User".to_string()));
}

#[tokio::test]
async fn test_get_profile_not_found() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();

    user_repo
        .expect_get_by_id()
        .with(eq(user_id))
        .returning(|_| Err(DomainError::UserNotFound));

    // Act
    let result = service.get_profile(user_id).await;

    // Assert
    assert!(result.is_err(), "Get profile should fail when user not found");
    assert!(matches!(result.unwrap_err(), DomainError::UserNotFound));
}

#[tokio::test]
async fn test_update_profile_success() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();
    let updated_at = created_at;

    let existing_user = User {
        id: user_id,
        email: "update@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        full_name: Some("Old Name".to_string()),
        created_at,
        updated_at,
    };

    let updated_user = User {
        id: user_id,
        email: "update@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        full_name: Some("New Name".to_string()),
        created_at,
        updated_at: chrono::Utc::now(),
    };

    let update_req = UpdateProfileRequest {
        full_name: Some("New Name".to_string()),
        email: None,
    };

    user_repo
        .expect_get_by_id()
        .with(eq(user_id))
        .returning(move |_| Ok(existing_user.clone()));

    user_repo.expect_update().returning(|_| Ok(()));

    // Act
    let result = service.update_profile(user_id, update_req).await;

    // Assert
    assert!(result.is_ok(), "Update profile should succeed");
    let response = result.unwrap();
    assert_eq!(response.full_name, Some("New Name".to_string()));
}

#[tokio::test]
async fn test_update_profile_email_already_exists() {
    // Arrange
    let (service, mut user_repo, _refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();

    let existing_user = User {
        id: user_id,
        email: "original@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        full_name: Some("User".to_string()),
        created_at,
        updated_at: created_at,
    };

    let update_req = UpdateProfileRequest {
        full_name: None,
        email: Some("taken@example.com".to_string()),
    };

    user_repo
        .expect_get_by_id()
        .with(eq(user_id))
        .returning(move |_| Ok(existing_user.clone()));

    user_repo
        .expect_exists_by_email()
        .with(eq("taken@example.com"))
        .returning(|_| Ok(true));

    // Act
    let result = service.update_profile(user_id, update_req).await;

    // Assert
    assert!(result.is_err(), "Update profile should fail when email is taken");
    assert!(matches!(result.unwrap_err(), DomainError::EmailAlreadyExists));
}

#[tokio::test]
async fn test_logout_success() {
    // Arrange
    let (service, _user_repo, mut refresh_repo, _task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();

    refresh_repo
        .expect_delete_by_token()
        .with(eq("refresh_token_to_delete"))
        .returning(|_| Ok(()));

    // Act
    let result = service.logout(user_id, "refresh_token_to_delete".to_string()).await;

    // Assert
    assert!(result.is_ok(), "Logout should succeed");
}

#[tokio::test]
async fn test_delete_account_success() {
    // Arrange
    let (service, mut user_repo, mut refresh_repo, mut task_repo) = create_mock_service().await;
    let user_id = Uuid::new_v4();

    refresh_repo
        .expect_delete_by_user_id()
        .with(eq(user_id))
        .returning(|_| Ok(1));

    task_repo
        .expect_delete_by_user()
        .with(eq(user_id))
        .returning(|_| Ok(5));

    user_repo
        .expect_delete()
        .with(eq(user_id))
        .returning(|_| Ok(()));

    // Act
    let result = service.delete_account(user_id).await;

    // Assert
    assert!(result.is_ok(), "Delete account should succeed");
}
