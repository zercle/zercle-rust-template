use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::internal::domain::error::DomainError;

/// Error handling middleware that converts DomainError to JSend responses
pub async fn error_handler_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let response = next.run(req).await;
    
    if response.status().is_server_error() {
        // Convert unhandled errors to JSend format
        let error_response = json!({
            "status": "error",
            "error": {
                "code": "INTERNAL_ERROR",
                "message": "An unexpected error occurred"
            }
        });
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
    }
    
    response
}

/// Convert DomainError to HTTP status code and JSend response
pub fn map_domain_error(error: DomainError) -> (StatusCode, serde_json::Value) {
    let (status, code, message) = match error {
        DomainError::Validation(msg) => {
            (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.to_string())
        }
        DomainError::UserNotFound => {
            (StatusCode::NOT_FOUND, "USER_NOT_FOUND", "User not found".to_string())
        }
        DomainError::TaskNotFound => {
            (StatusCode::NOT_FOUND, "TASK_NOT_FOUND", "Task not found".to_string())
        }
        DomainError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "INVALID_CREDENTIALS",
            "Invalid credentials".to_string(),
        ),
        DomainError::TokenExpired => (
            StatusCode::UNAUTHORIZED,
            "TOKEN_EXPIRED",
            "Token has expired".to_string(),
        ),
        DomainError::TokenInvalid => (
            StatusCode::UNAUTHORIZED,
            "TOKEN_INVALID",
            "Invalid token".to_string(),
        ),
        DomainError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Unauthorized".to_string(),
        ),
        DomainError::Forbidden => (
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "Forbidden: insufficient permissions".to_string(),
        ),
        DomainError::UserAlreadyExists => (
            StatusCode::CONFLICT,
            "USER_ALREADY_EXISTS",
            "User already exists".to_string(),
        ),
        DomainError::EmailAlreadyExists => (
            StatusCode::CONFLICT,
            "EMAIL_ALREADY_EXISTS",
            "Email already exists".to_string(),
        ),
        DomainError::Database(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            msg.to_string(),
        ),
        DomainError::Internal => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal server error".to_string(),
        ),
    };

    let body = json!({
        "status": "error",
        "error": {
            "code": code,
            "message": message
        }
    });

    (status, body)
}
