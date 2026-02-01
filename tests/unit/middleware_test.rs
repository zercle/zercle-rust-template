//! Unit tests for HTTP middleware
//!
//! Tests authentication middleware, rate limiting, request ID injection,
// and error handling middleware.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use tower::ServiceExt;

use zercle_rust_template::internal::domain::error::DomainError;
use zercle_rust_template::internal::infrastructure::http::middleware::{auth, request_id};

/// Mock JWT generator for middleware testing
#[derive(Debug, Clone)]
struct MockJwtGeneratorForMiddleware;

impl zercle_rust_template::internal::domain::user::traits::JwtGenerator for MockJwtGeneratorForMiddleware {
    fn generate_access_token(
        &self,
        _user_id: uuid::Uuid,
        _email: &str,
    ) -> Result<String, DomainError> {
        Ok("mock_access_token".to_string())
    }

    fn generate_refresh_token(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<(String, chrono::DateTime<chrono::Utc>), DomainError> {
        use chrono::{Duration, Utc};
        Ok(("mock_refresh_token".to_string(), Utc::now() + Duration::days(7)))
    }

    fn validate_access_token(&self, token: &str) -> Result<(uuid::Uuid, String), DomainError> {
        if token == "valid_token" {
            Ok((uuid::Uuid::new_v4(), "test@example.com".to_string()))
        } else if token == "expired_token" {
            Err(DomainError::TokenExpired)
        } else {
            Err(DomainError::TokenInvalid)
        }
    }
}

/// Create a test app with auth middleware
fn create_test_app() -> Router {
    let jwt = Arc::new(MockJwtGeneratorForMiddleware);

    Router::new()
        .route("/public", get(|| async { "public" }))
        .route(
            "/protected",
            get(|| async { "protected" }),
        )
        .layer(axum::middleware::from_fn(move |req, next| {
            let jwt = jwt.clone();
            auth::auth_middleware(jwt, req, next)
        }))
}

/// Create a test app with request ID middleware
fn create_test_app_with_request_id() -> Router {
    Router::new()
        .route("/test", get(|| async { "test" }))
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
}

#[tokio::test]
async fn test_auth_middleware_valid_token() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_middleware_missing_header() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_invalid_format() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "InvalidFormat token")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_missing_bearer_prefix() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "valid_token")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_expired_token() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer expired_token")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_invalid_token() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/protected")
        .header("authorization", "Bearer completely_invalid_token")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_request_id_middleware_adds_id() {
    // Arrange
    let app = create_test_app_with_request_id();

    let request = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    // Check that request-id header is present
    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("Request ID header should be present");

    assert!(!request_id.is_empty(), "Request ID should not be empty");
}

#[tokio::test]
async fn test_request_id_middleware_different_ids() {
    // Arrange
    let app = create_test_app_with_request_id();

    // Make multiple requests
    let request1 = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let request2 = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Act
    let response1 = app.clone().oneshot(request1).await.unwrap();
    let response2 = app.oneshot(request2).await.unwrap();

    // Assert
    let id1 = response1
        .headers()
        .get("x-request-id")
        .expect("Request ID header should be present");
    let id2 = response2
        .headers()
        .get("x-request-id")
        .expect("Request ID header should be present");

    // Request IDs should be different (UUIDs)
    assert_ne!(id1, id2, "Each request should get a unique ID");
}

#[tokio::test]
async fn test_public_route_without_auth() {
    // Arrange
    let app = create_test_app();

    let request = Request::builder()
        .uri("/public")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}
