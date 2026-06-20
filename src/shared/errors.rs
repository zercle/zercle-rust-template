//! Shared boundary error type. Mirrors `internal/shared/errors/app_error.go` (structure.md §6).
//!
//! - `AppError::http_status()` → `axum::http::StatusCode`
//! - `AppError::grpc_code()` → `tonic::Code`
//! - `impl IntoResponse for AppError` → JSON `{"error": CODE, "message": MSG}` with the status.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tonic::Code as GrpcCode;

/// Shared, transport-agnostic error used at the HTTP / gRPC boundary.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("resource not found")]
    NotFound { cause: Option<anyhow::Error> },
    #[error("invalid input")]
    InvalidInput { cause: Option<anyhow::Error> },
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict")]
    Conflict,
    #[error("request canceled")]
    Canceled,
    #[error("deadline exceeded")]
    DeadlineExceeded,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::InvalidInput { .. } => "INVALID_INPUT",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict => "CONFLICT",
            Self::Canceled => "CANCELED",
            Self::DeadlineExceeded => "DEADLINE_EXCEEDED",
            Self::Internal { .. } => "INTERNAL",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "resource not found",
            Self::InvalidInput { .. } => "invalid input",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::Conflict => "conflict",
            Self::Canceled => "request canceled",
            Self::DeadlineExceeded => "deadline exceeded",
            Self::Internal { .. } => "internal error",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Canceled => {
                StatusCode::from_u16(499).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn grpc_code(&self) -> GrpcCode {
        match self {
            Self::NotFound { .. } => GrpcCode::NotFound,
            Self::InvalidInput { .. } => GrpcCode::InvalidArgument,
            Self::Unauthorized => GrpcCode::Unauthenticated,
            Self::Forbidden => GrpcCode::PermissionDenied,
            Self::Conflict => GrpcCode::AlreadyExists,
            Self::Canceled => GrpcCode::Cancelled,
            Self::DeadlineExceeded => GrpcCode::DeadlineExceeded,
            Self::Internal { .. } => GrpcCode::Internal,
        }
    }

    pub fn to_grpc_status(&self) -> tonic::Status {
        let mut s = tonic::Status::new(self.grpc_code(), self.message());
        if let Some(cause) = self.cause() {
            s = tonic::Status::with_details(
                self.grpc_code(),
                self.message(),
                axum::body::Bytes::from(format!("{:#}", cause)),
            );
        }
        s
    }

    pub fn cause(&self) -> Option<&anyhow::Error> {
        match self {
            Self::NotFound { cause } | Self::InvalidInput { cause } | Self::Internal { cause } => {
                cause.as_ref()
            }
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(ErrorBody {
            error: self.code(),
            message: self.message(),
        });
        (self.http_status(), body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_per_variant() {
        assert_eq!(
            AppError::NotFound { cause: None }.http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::InvalidInput { cause: None }.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AppError::Forbidden.http_status(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::Conflict.http_status(), StatusCode::CONFLICT);
        assert_eq!(
            AppError::Canceled.http_status(),
            StatusCode::from_u16(499).unwrap()
        );
        assert_eq!(
            AppError::DeadlineExceeded.http_status(),
            StatusCode::GATEWAY_TIMEOUT
        );
        assert_eq!(
            AppError::Internal { cause: None }.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn grpc_code_per_variant() {
        assert_eq!(
            AppError::NotFound { cause: None }.grpc_code(),
            GrpcCode::NotFound
        );
        assert_eq!(
            AppError::InvalidInput { cause: None }.grpc_code(),
            GrpcCode::InvalidArgument
        );
        assert_eq!(
            AppError::Unauthorized.grpc_code(),
            GrpcCode::Unauthenticated
        );
        assert_eq!(AppError::Forbidden.grpc_code(), GrpcCode::PermissionDenied);
        assert_eq!(AppError::Conflict.grpc_code(), GrpcCode::AlreadyExists);
        assert_eq!(AppError::Canceled.grpc_code(), GrpcCode::Cancelled);
        assert_eq!(
            AppError::DeadlineExceeded.grpc_code(),
            GrpcCode::DeadlineExceeded
        );
        assert_eq!(
            AppError::Internal { cause: None }.grpc_code(),
            GrpcCode::Internal
        );
    }

    #[test]
    fn code_and_message_are_stable() {
        assert_eq!(AppError::NotFound { cause: None }.code(), "NOT_FOUND");
        assert_eq!(
            AppError::Internal { cause: None }.message(),
            "internal error"
        );
    }
}
