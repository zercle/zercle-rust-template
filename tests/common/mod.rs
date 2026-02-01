//! Test utilities and helper functions
//!
//! This module provides common test utilities, mock helpers, and factory functions
//! for creating test data across all test modules.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use zercle_rust_template::internal::domain::{
    task::entity::{Task, TaskPriority, TaskStatus},
    user::entity::User,
};

/// Create a test user with default values
#[allow(dead_code)]
pub fn create_test_user() -> User {
    let id = Uuid::new_v4();
    User::new(
        id.clone(),
        format!("test_{}@example.com", id.to_string()[..8]),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    )
}

/// Create a test user with a specific email
#[allow(dead_code)]
pub fn create_test_user_with_email(email: &str) -> User {
    let id = Uuid::new_v4();
    User::new(
        id,
        email.to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    )
}

/// Create a test task with default values
#[allow(dead_code)]
pub fn create_test_task(user_id: Uuid) -> Task {
    let id = Uuid::new_v4();
    Task::new(
        id,
        user_id,
        format!("Test Task {}", id.to_string()[..8]),
        Some("Test task description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    )
}

/// Create a test task with custom values
#[allow(dead_code)]
pub fn create_test_task_with_values(
    user_id: Uuid,
    title: &str,
    status: TaskStatus,
    priority: TaskPriority,
) -> Task {
    let id = Uuid::new_v4();
    Task::new(
        id,
        user_id,
        title.to_string(),
        Some("Test task description".to_string()),
        status,
        priority,
        None,
    )
}

/// Get a test JWT secret
#[allow(dead_code)]
pub fn get_test_jwt_secret() -> &'static [u8] {
    b"test-secret-key-for-unit-testing-only"
}

/// Get a different JWT secret for testing token validation failures
#[allow(dead_code)]
pub fn get_different_jwt_secret() -> &'static [u8] {
    b"different-secret-key-for-validation-testing"
}

/// Create a datetime in the past for testing expired tokens
#[allow(dead_code)]
pub fn create_expired_datetime() -> DateTime<Utc> {
    Utc::now() - chrono::Duration::hours(1)
}

/// Create a datetime in the future for testing valid tokens
#[allow(dead_code)]
pub fn create_future_datetime() -> DateTime<Utc> {
    Utc::now() + chrono::Duration::hours(1)
}

/// Generate a random alphanumeric string for test data
#[allow(dead_code)]
pub fn generate_random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

/// Valid test password that meets validation requirements
#[allow(dead_code)]
pub fn get_valid_test_password() -> String {
    "SecureP@ss123!".to_string()
}

/// Invalid test password (too short)
#[allow(dead_code)]
pub fn get_invalid_short_password() -> String {
    "short".to_string()
}
