use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Comprehensive domain error type covering all possible error scenarios
#[derive(Debug, Error)]
pub enum DomainError {
    // Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    // Not found errors
    #[error("User not found")]
    UserNotFound,
    #[error("Task not found")]
    TaskNotFound,

    // Authentication errors
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Token expired")]
    TokenExpired,
    #[error("Token invalid")]
    TokenInvalid,

    // Authorization errors
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden: insufficient permissions")]
    Forbidden,

    // Conflict errors
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Email already exists")]
    EmailAlreadyExists,

    // Internal errors
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for DomainError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            DomainError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.to_string()),
            DomainError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", "User not found".to_string()),
            DomainError::TaskNotFound => (StatusCode::NOT_FOUND, "TASK_NOT_FOUND", "Task not found".to_string()),
            DomainError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS", "Invalid credentials".to_string())
            }
            DomainError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED", "Token has expired".to_string()),
            DomainError::TokenInvalid => (StatusCode::UNAUTHORIZED, "TOKEN_INVALID", "Invalid token".to_string()),
            DomainError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Unauthorized".to_string()),
            DomainError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "Forbidden".to_string()),
            DomainError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "USER_ALREADY_EXISTS", "User already exists".to_string())
            }
            DomainError::EmailAlreadyExists => {
                (StatusCode::CONFLICT, "EMAIL_ALREADY_EXISTS", "Email already exists".to_string())
            }
            DomainError::Database(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", msg.to_string())
            }
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

        (status, Json(body)).into_response()
    }
}
