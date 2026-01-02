//! HTTP Handler Tests - Simplified
//!
//! This module contains basic unit tests for HTTP handlers and responses.

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use uuid::Uuid;
use zercle_rust_template::domain::entities::{
    CreateTaskRequest, CreateUserRequest, LoginRequest, TaskPriority, TaskStatus,
    UpdateTaskRequest, User,
};
use zercle_rust_template::infrastructure::http::handlers::ApiResponse;

// ============================================================================
// ApiResponse Tests
// ============================================================================

mod api_response_tests {
    use super::*;

    /// Test ApiResponse::success
    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::<String>::success("test data".to_string());

        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.unwrap(), "test data");
        assert!(response.message.is_none());
        assert!(response.error.is_none());
    }

    /// Test ApiResponse::with_message
    #[test]
    fn test_api_response_with_message() {
        let response =
            ApiResponse::<String>::with_message(Some("data".to_string()), "Operation successful");

        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.message.unwrap(), "Operation successful");
        assert!(response.error.is_none());
    }

    /// Test ApiResponse::error
    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("Something went wrong");

        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.message.is_none());
        assert_eq!(response.error.unwrap(), "Something went wrong");
    }

    /// Test ApiResponse with None data
    #[test]
    fn test_api_response_none_data() {
        let response = ApiResponse::<String>::success("".to_string());
        assert!(response.data.is_some());

        let response = ApiResponse::<String>::success("".to_string());
        assert!(response.data.is_some());
    }
}

// ============================================================================
// Entity Factory Tests
// ============================================================================

mod entity_factory_tests {
    use super::*;

    /// Test CreateUserRequest factory
    #[test]
    fn test_create_user_request() {
        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: Some("+1234567890".to_string()),
        };

        assert_eq!(req.email, "test@example.com");
        assert_eq!(req.password, "Password123!");
        assert_eq!(req.full_name, Some("Test User".to_string()));
        assert_eq!(req.phone, Some("+1234567890".to_string()));
    }

    /// Test LoginRequest factory
    #[test]
    fn test_login_request() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(req.email, "test@example.com");
        assert_eq!(req.password, "password123");
    }

    /// Test CreateTaskRequest factory
    #[test]
    fn test_create_task_request() {
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: Some(TaskPriority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        };

        assert_eq!(req.title, "Test Task");
        assert_eq!(req.description, Some("Description".to_string()));
        assert_eq!(req.priority, Some(TaskPriority::High));
    }

    /// Test UpdateTaskRequest factory
    #[test]
    fn test_update_task_request() {
        let req = UpdateTaskRequest {
            title: Some("Updated Title".to_string()),
            description: Some("Updated Description".to_string()),
            status: Some(TaskStatus::Completed),
            priority: Some(TaskPriority::Low),
            due_date: None,
        };

        assert_eq!(req.title, Some("Updated Title".to_string()));
        assert_eq!(req.status, Some(TaskStatus::Completed));
        assert!(req.has_updates());
    }

    /// Test UpdateTaskRequest empty
    #[test]
    fn test_update_task_request_empty() {
        let req = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            due_date: None,
        };

        assert!(!req.has_updates());
    }
}

// ============================================================================
// StatusCode Tests
// ============================================================================

mod status_code_tests {
    use super::*;

    /// Test common status codes
    #[test]
    fn test_status_codes() {
        assert_eq!(StatusCode::OK.as_u16(), 200);
        assert_eq!(StatusCode::CREATED.as_u16(), 201);
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::UNAUTHORIZED.as_u16(), 401);
        assert_eq!(StatusCode::FORBIDDEN.as_u16(), 403);
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
        assert_eq!(StatusCode::CONFLICT.as_u16(), 409);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }

    /// Test status code categories
    #[test]
    fn test_status_code_categories() {
        assert!(StatusCode::OK.is_success());
        assert!(StatusCode::CREATED.is_success());
        assert!(StatusCode::BAD_REQUEST.is_client_error());
        assert!(StatusCode::NOT_FOUND.is_client_error());
        assert!(StatusCode::INTERNAL_SERVER_ERROR.is_server_error());
        assert!(StatusCode::UNAUTHORIZED.is_client_error());
    }
}
