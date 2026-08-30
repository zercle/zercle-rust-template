//! Shared boundary error type. Mirrors Go `internal/platform/errors`.
//!
//! - `AppError::http_status()` → `axum::http::StatusCode`
//! - `AppError::grpc_code()` → `tonic::Code`
//! - `impl IntoResponse for AppError` → JSON `{"error": CODE, "message": MSG}` with the status.
//!
//! The machine-readable codes are exposed as [`errcodes`] constants and
//! published outward via `crate::api::v1::errcodes` so other services can
//! interpret error envelopes without importing server internals.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tonic::Code as GrpcCode;

/// Stable machine-readable error codes carried on the wire (the HTTP JSON
/// `error` field). Published outward via `crate::api::v1::errcodes`; feature
/// domains map their sentinels onto these at the composition edge (each
/// feature's `di`).
pub mod errcodes {
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const CONFLICT: &str = "CONFLICT";
    pub const CANCELED: &str = "CANCELED";
    pub const DEADLINE_EXCEEDED: &str = "DEADLINE_EXCEEDED";
    pub const INTERNAL: &str = "INTERNAL";
}

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
            Self::NotFound { .. } => errcodes::NOT_FOUND,
            Self::InvalidInput { .. } => errcodes::INVALID_INPUT,
            Self::Unauthorized => errcodes::UNAUTHORIZED,
            Self::Forbidden => errcodes::FORBIDDEN,
            Self::Conflict => errcodes::CONFLICT,
            Self::Canceled => errcodes::CANCELED,
            Self::DeadlineExceeded => errcodes::DEADLINE_EXCEEDED,
            Self::Internal { .. } => errcodes::INTERNAL,
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

    /// Convert to a `tonic::Status` for transport. For the `Internal` variant
    /// the cause (which may contain raw DB / SQL details) is intentionally
    /// redacted from the wire message to avoid leaking internals to clients
    /// (CWE-209); the cause is already logged at the error-construction call
    /// site. For client-facing variants (`NotFound`, `InvalidInput`) the
    /// cause is safe and useful, so it is appended to the message.
    ///
    /// The gRPC `grpc-status-details-bin` trailer (populated by
    /// `Status::with_details`) MUST contain a serialized `google.rpc.Status`
    /// protobuf message; raw text bytes violate the spec and break standard
    /// clients, so we append the cause to the human-readable message instead.
    pub fn to_grpc_status(&self) -> tonic::Status {
        if matches!(self, Self::Internal { .. }) {
            return tonic::Status::new(self.grpc_code(), self.message());
        }
        if let Some(cause) = self.cause() {
            tonic::Status::new(self.grpc_code(), format!("{}: {:#}", self.message(), cause))
        } else {
            tonic::Status::new(self.grpc_code(), self.message())
        }
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

    #[test]
    fn grpc_status_internal_does_not_leak_cause() {
        let err = AppError::Internal {
            cause: Some(anyhow::anyhow!("relation users does not exist")),
        };
        let status = err.to_grpc_status();
        assert_eq!(status.code(), GrpcCode::Internal);
        assert_eq!(status.message(), "internal error");
        assert!(
            !status.message().contains("relation users"),
            "internal error message leaked SQL detail: {}",
            status.message()
        );
        assert!(
            !status.message().contains("does not exist"),
            "internal error message leaked SQL detail: {}",
            status.message()
        );
    }

    #[test]
    fn grpc_status_invalid_input_appends_cause() {
        let err = AppError::InvalidInput {
            cause: Some(anyhow::anyhow!("field email is required")),
        };
        let status = err.to_grpc_status();
        assert_eq!(status.code(), GrpcCode::InvalidArgument);
        assert!(
            status.message().contains("field email is required"),
            "expected cause to appear in InvalidInput message: {}",
            status.message()
        );
    }

    #[test]
    fn grpc_status_not_found_appends_cause() {
        let err = AppError::NotFound {
            cause: Some(anyhow::anyhow!("user 42 not found in db")),
        };
        let status = err.to_grpc_status();
        assert_eq!(status.code(), GrpcCode::NotFound);
        assert!(
            status.message().contains("user 42 not found in db"),
            "expected cause to appear in NotFound message: {}",
            status.message()
        );
    }
}
