//! Unit tests for entity validation logic
//!
//! These tests verify the validation logic in entities without requiring
//! a database connection.

use chrono::{Duration, Utc};
use uuid::Uuid;
use zercle_rust_template::domain::entities::{
    CreateTaskRequest, CreateUserRequest, LoginRequest, TaskPriority, TaskStatus,
    UpdateTaskRequest, UpdateUserRequest, UserValidationError,
};
use zercle_rust_template::domain::task::TaskValidationError;

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

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    /// Test CreateTaskRequest with max length title (255 characters)
    #[test]
    fn test_create_task_request_max_length_title() {
        let max_length_title = "a".repeat(255);
        let request = CreateTaskRequest {
            title: max_length_title,
            description: None,
            priority: None,
            due_date: Some(Utc::now() + Duration::days(1)),
        };

        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with title exceeding max length by 1
    #[test]
    fn test_create_task_request_title_exceeds_max_by_one() {
        let over_length_title = "a".repeat(256);
        let request = CreateTaskRequest {
            title: over_length_title,
            description: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_err());
    }

    /// Test CreateTaskRequest with special characters in title
    #[test]
    fn test_create_task_request_special_characters() {
        let request = CreateTaskRequest {
            title: "Task with special chars @#$%^&*()_+=[]{}|;:".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        // Special characters are allowed, should pass validation
        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with unicode characters in title
    #[test]
    fn test_create_task_request_unicode_characters() {
        let request = CreateTaskRequest {
            title: "Tâsk with ünïcödé chàrâctërs 你好".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        // Unicode characters are allowed, should pass validation
        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with whitespace only title
    #[test]
    fn test_create_task_request_whitespace_title() {
        let request = CreateTaskRequest {
            title: "   ".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        // Whitespace-only title is technically valid (min length 1, whitespace counts)
        // The business logic may want to reject this, but validation passes
        let result = request.validate_request();
        assert!(result.is_ok());
    }

    /// Test TaskStatus - verify all status transitions
    #[test]
    fn test_task_status_all_transitions() {
        // From Pending, can go to any status
        let pending = TaskStatus::Pending;
        assert!(!pending.is_terminal());
        assert!(pending.is_active());

        // From InProgress, can go to any status
        let in_progress = TaskStatus::InProgress;
        assert!(!in_progress.is_terminal());
        assert!(in_progress.is_active());

        // From Completed, is terminal
        let completed = TaskStatus::Completed;
        assert!(completed.is_terminal());
        assert!(!completed.is_active());

        // From Cancelled, is terminal
        let cancelled = TaskStatus::Cancelled;
        assert!(cancelled.is_terminal());
        assert!(!cancelled.is_active());
    }

    /// Test TaskStatus - verify status is not equal to others
    #[test]
    fn test_task_status_inequality() {
        assert_ne!(TaskStatus::Pending, TaskStatus::InProgress);
        assert_ne!(TaskStatus::Pending, TaskStatus::Completed);
        assert_ne!(TaskStatus::Pending, TaskStatus::Cancelled);
        assert_ne!(TaskStatus::InProgress, TaskStatus::Completed);
        assert_ne!(TaskStatus::InProgress, TaskStatus::Cancelled);
        assert_ne!(TaskStatus::Completed, TaskStatus::Cancelled);
    }

    /// Test TaskPriority - verify priority ordering
    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Low.value() < TaskPriority::Medium.value());
        assert!(TaskPriority::Medium.value() < TaskPriority::High.value());
        assert!(TaskPriority::High.value() < TaskPriority::Urgent.value());
        assert!(TaskPriority::Urgent.value() > TaskPriority::Low.value());
    }

    /// Test TaskPriority - verify from_value edge cases
    #[test]
    fn test_task_priority_from_value_edge_cases() {
        // Valid values
        assert!(TaskPriority::from_value(0).is_none());
        assert!(TaskPriority::from_value(1).is_some());
        assert!(TaskPriority::from_value(4).is_some());
        assert!(TaskPriority::from_value(5).is_none());
        assert!(TaskPriority::from_value(255).is_none());
        assert!(TaskPriority::from_value(u8::MAX).is_none());
    }

    /// Test CreateTaskRequest with due date exactly at current time
    #[test]
    fn test_create_task_request_due_date_at_now() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: Some(Utc::now()),
        };

        // Due date at exactly now should fail (must be in future)
        let result = request.validate_request();
        assert!(result.is_err());
    }

    /// Test CreateTaskRequest with due date 1 second in future
    #[test]
    fn test_create_task_request_due_date_one_second_future() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: Some(Utc::now() + Duration::seconds(1)),
        };

        // Due date 1 second in future should pass
        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with very long description (boundary)
    #[test]
    fn test_create_task_request_max_length_description() {
        let max_length_desc = "a".repeat(5000);
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: Some(max_length_desc),
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_ok());
    }

    /// Test CreateTaskRequest with description exceeding max length
    #[test]
    fn test_create_task_request_description_exceeds_max() {
        let over_length_desc = "a".repeat(5001);
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: Some(over_length_desc),
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_err());
    }

    /// Test CreateTaskRequest with all priority levels
    #[test]
    fn test_create_task_request_all_priorities() {
        for priority in [
            TaskPriority::Low,
            TaskPriority::Medium,
            TaskPriority::High,
            TaskPriority::Urgent,
        ] {
            let request = CreateTaskRequest {
                title: "Test Task".to_string(),
                description: None,
                priority: Some(priority),
                due_date: None,
            };
            assert!(request.validate_request().is_ok());
        }
    }

    /// Test CreateTaskRequest with all statuses in update
    #[test]
    fn test_update_task_request_all_statuses() {
        for status in TaskStatus::all() {
            let request = UpdateTaskRequest {
                title: None,
                description: None,
                status: Some(status),
                priority: None,
                due_date: None,
            };
            // All statuses are valid in update request
            assert!(request.validate_request().is_ok());
            assert!(request.has_updates());
        }
    }
}
