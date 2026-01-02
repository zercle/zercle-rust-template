//! Unit tests for entity validation logic
//!
//! These tests verify the validation logic in entities without requiring
//! a database connection.

use zercle_rust_template::domain::entities::{
    CreateUserRequest, CreateTaskRequest, LoginRequest, TaskStatus, TaskPriority,
    UpdateUserRequest, UpdateTaskRequest, UserValidationError, TaskValidationError,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

mod user_validation_tests {
    use super::*;

    /// Test CreateUserRequest with valid data
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

    /// Test CreateUserRequest with invalid email
    #[test]
    fn test_create_user_request_invalid_email() {
        let request = CreateUserRequest {
            email: "invalid-email".to_string(),
            password: "Password123!".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, UserValidationError::EmailError(_)));
        }
    }

    /// Test CreateUserRequest with password too short
    #[test]
    fn test_create_user_request_password_too_short() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Short1!".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
    }

    /// Test CreateUserRequest with missing uppercase
    #[test]
    fn test_create_user_request_password_no_uppercase() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "password123!".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, UserValidationError::PasswordError(msg) 
                if msg.contains("uppercase")));
        }
    }

    /// Test CreateUserRequest with missing lowercase
    #[test]
    fn test_create_user_request_password_no_lowercase() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "PASSWORD123!".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, UserValidationError::PasswordError(msg) 
                if msg.contains("lowercase")));
        }
    }

    /// Test CreateUserRequest with missing digit
    #[test]
    fn test_create_user_request_password_no_digit() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "PasswordTest!".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, UserValidationError::PasswordError(msg) 
                if msg.contains("number")));
        }
    }

    /// Test CreateUserRequest with missing special character
    #[test]
    fn test_create_user_request_password_no_special() {
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            full_name: None,
            phone: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, UserValidationError::PasswordError(msg) 
                if msg.contains("special")));
        }
    }

    /// Test LoginRequest with valid data
    #[test]
    fn test_login_request_valid() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert!(request.validate_request().is_ok());
    }

    /// Test LoginRequest with invalid email
    #[test]
    fn test_login_request_invalid_email() {
        let request = LoginRequest {
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };

        assert!(request.validate_request().is_err());
    }

    /// Test LoginRequest with empty password
    #[test]
    fn test_login_request_empty_password() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };

        assert!(request.validate_request().is_err());
    }

    /// Test UpdateUserRequest with valid data
    #[test]
    fn test_update_user_request_valid() {
        let request = UpdateUserRequest {
            full_name: Some("Jane Doe".to_string()),
            phone: Some("+0987654321".to_string()),
        };

        assert!(request.validate_request().is_ok());
        assert!(request.has_updates());
    }

    /// Test UpdateUserRequest with empty request
    #[test]
    fn test_update_user_request_empty() {
        let request = UpdateUserRequest {
            full_name: None,
            phone: None,
        };

        assert!(request.validate_request().is_ok());
        assert!(!request.has_updates());
    }

    /// Test UpdateUserRequest with full_name too long
    #[test]
    fn test_update_user_request_full_name_too_long() {
        let request = UpdateUserRequest {
            full_name: Some("a".repeat(256)),
            phone: None,
        };

        assert!(request.validate_request().is_err());
    }
}

mod task_validation_tests {
    use super::*;

    /// Test CreateTaskRequest with valid data
    #[test]
    fn test_create_task_request_valid() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Test Description".to_string()),
            priority: Some(TaskPriority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        };

        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with empty title
    #[test]
    fn test_create_task_request_empty_title() {
        let request = CreateTaskRequest {
            title: "".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, TaskValidationError::TitleError(_)));
        }
    }

    /// Test CreateTaskRequest with title too long
    #[test]
    fn test_create_task_request_title_too_long() {
        let request = CreateTaskRequest {
            title: "a".repeat(256),
            description: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_err());
    }

    /// Test CreateTaskRequest with past due date
    #[test]
    fn test_create_task_request_past_due_date() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: Some(Utc::now() - Duration::days(1)),
        };

        let result = request.validate_request();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, TaskValidationError::DueDateError(msg) 
                if msg.contains("future")));
        }
    }

    /// Test CreateTaskRequest with default priority
    #[test]
    fn test_create_task_request_default_priority() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        assert_eq!(request.get_priority(), TaskPriority::Medium);
    }

    /// Test UpdateTaskRequest with valid data
    #[test]
    fn test_update_task_request_valid() {
        let request = UpdateTaskRequest {
            title: Some("Updated Task".to_string()),
            description: Some("Updated Description".to_string()),
            status: Some(TaskStatus::Completed),
            priority: Some(TaskPriority::Urgent),
            due_date: Some(Utc::now() + Duration::days(2)),
        };

        assert!(request.validate_request().is_ok());
        assert!(request.has_updates());
    }

    /// Test UpdateTaskRequest with empty request
    #[test]
    fn test_update_task_request_empty() {
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_ok());
        assert!(!request.has_updates());
    }

    /// Test TaskStatus enum
    #[test]
    fn test_task_status_all() {
        let statuses = TaskStatus::all();
        assert_eq!(statuses.len(), 4);
        assert!(statuses.contains(&TaskStatus::Pending));
        assert!(statuses.contains(&TaskStatus::InProgress));
        assert!(statuses.contains(&TaskStatus::Completed));
        assert!(statuses.contains(&TaskStatus::Cancelled));
    }

    /// Test TaskStatus is_terminal
    #[test]
    fn test_task_status_is_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    /// Test TaskStatus is_active
    #[test]
    fn test_task_status_is_active() {
        assert!(TaskStatus::Pending.is_active());
        assert!(TaskStatus::InProgress.is_active());
        assert!(!TaskStatus::Completed.is_active());
        assert!(!TaskStatus::Cancelled.is_active());
    }

    /// Test TaskPriority enum
    #[test]
    fn test_task_priority_all() {
        let priorities = TaskPriority::all();
        assert_eq!(priorities.len(), 4);
        assert!(priorities.contains(&TaskPriority::Low));
        assert!(priorities.contains(&TaskPriority::Medium));
        assert!(priorities.contains(&TaskPriority::High));
        assert!(priorities.contains(&TaskPriority::Urgent));
    }

    /// Test TaskPriority value
    #[test]
    fn test_task_priority_value() {
        assert_eq!(TaskPriority::Low.value(), 1);
        assert_eq!(TaskPriority::Medium.value(), 2);
        assert_eq!(TaskPriority::High.value(), 3);
        assert_eq!(TaskPriority::Urgent.value(), 4);
    }

    /// Test TaskPriority from_value
    #[test]
    fn test_task_priority_from_value() {
        assert_eq!(TaskPriority::from_value(1), Some(TaskPriority::Low));
        assert_eq!(TaskPriority::from_value(2), Some(TaskPriority::Medium));
        assert_eq!(TaskPriority::from_value(3), Some(TaskPriority::High));
        assert_eq!(TaskPriority::from_value(4), Some(TaskPriority::Urgent));
        assert_eq!(TaskPriority::from_value(5), None);
    }
}
