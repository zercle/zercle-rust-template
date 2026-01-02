//! Authentication middleware
//!
//! This module handles JWT validation and user authentication.

use crate::config::Settings;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Json,
};
use axum::http::StatusCode;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

/// Auth state for middleware
#[derive(Clone)]
pub struct AuthState {
    pub jwt_secret: Arc<String>,
}

impl AuthState {
    pub fn new(settings: &Settings) -> Self {
        Self {
            jwt_secret: Arc::new(settings.jwt.secret.clone()),
        }
    }
}

/// Extension key for user ID in request
#[derive(Clone, Copy, Debug)]
pub struct UserId(pub Uuid);

/// Error response structure
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub success: bool,
    pub error: String,
}

/// Auth middleware function
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if auth_header.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let auth_header = auth_header.unwrap();
    let token = if let Some(prefix) = auth_header.strip_prefix("Bearer ") {
        prefix
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let secret = state.jwt_secret.as_str();
    let validation = Validation::default();
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = match decode::<JwtClaims>(token, &decoding_key, &validation) {
        Ok(data) => data,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    let user_id = match Uuid::parse_str(&token_data.claims.sub) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    req.extensions_mut().insert(UserId(user_id));
    Ok(next.run(req).await)
}

/// Helper function to create auth error response
pub fn create_auth_error_response(message: &str) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(AuthErrorResponse {
            success: false,
            error: message.to_string(),
        }),
    )
}
