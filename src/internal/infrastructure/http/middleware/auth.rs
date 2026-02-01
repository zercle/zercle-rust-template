use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::internal::{
    domain::error::DomainError,
    domain::user::traits::JwtGenerator,
};

/// Auth context extracted from JWT token
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub email: String,
}

/// Extension key for auth context
pub const AUTH_CONTEXT_EXTENSION: &str = "auth_context";

impl AuthContext {
    /// Create a new auth context
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self { user_id, email }
    }
}

/// JWT authentication middleware
pub async fn auth_middleware(
    jwt_service: Arc<dyn JwtGenerator + Send + Sync>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, DomainError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.trim_start_matches("Bearer ").to_string()
        }
        Some(_) => {
            let response = Json(json!({
                "status": "error",
                "error": {
                    "code": "INVALID_TOKEN_FORMAT",
                    "message": "Authorization header must be Bearer token"
                }
            }));
            return Ok((StatusCode::UNAUTHORIZED, response).into_response());
        }
        None => {
            let response = Json(json!({
                "status": "error",
                "error": {
                    "code": "MISSING_TOKEN",
                    "message": "Authorization header is required"
                }
            }));
            return Ok((StatusCode::UNAUTHORIZED, response).into_response());
        }
    };

    let (user_id, email) = jwt_service.validate_access_token(&token)?;

    let auth_context = AuthContext::new(user_id, email);
    req.extensions_mut().insert(auth_context);

    Ok(next.run(req).await)
}

/// Helper function to extract auth context from request
pub fn require_auth(req: &axum::extract::Request) -> Result<AuthContext, Response> {
    req.extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "error": {
                        "code": "UNAUTHORIZED",
                        "message": "Authentication required"
                    }
                })),
            )
                .into_response()
        })
}
