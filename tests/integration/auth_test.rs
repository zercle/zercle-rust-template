//! Integration tests for authentication endpoints
//!
//! These tests verify the HTTP API layer for authentication operations
//! including registration, login, token refresh, and protected routes.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use zercle_rust_template::internal::infrastructure::{
    config::Config,
    di::Container,
    http::create_router,
};

use crate::integration::setup_test_container;

/// Test user data for integration tests
fn get_test_register_request() -> serde_json::Value {
    json!({
        "email": "integration_test@example.com",
        "password": "SecureP@ss123!",
        "full_name": "Integration Test User"
    })
}

fn get_test_login_request() -> serde_json::Value {
    json!({
        "email": "integration_test@example.com",
        "password": "SecureP@ss123!"
    })
}

/// Create a test container with test configuration
async fn create_test_app() -> (axum::Router, Arc<Container>) {
    let container = setup_test_container().await;
    let app = create_router(container.clone());
    (app, container)
}

#[tokio::test]
async fn test_register_endpoint_success() {
    // This test requires a running database
    // Skip if DATABASE_URL is not set
    if std::env::var("DATABASE_URL").is_err() {
        return; // Skip test in CI without database
    }

    // Arrange
    let (app, _container) = create_test_app().await;

    let unique_email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    let register_body = json!({
        "email": unique_email,
        "password": "SecureP@ss123!",
        "full_name": "Test User"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Registration should return 201 Created"
    );
}

#[tokio::test]
async fn test_register_endpoint_validation_error() {
    // This test can run without a database (just validation check)
    // Arrange
    let (app, _container) = create_test_app().await;

    let register_body = json!({
        "email": "invalid-email", // Invalid email
        "password": "short", // Too short
        "full_name": "Test User"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Invalid registration data should return 400"
    );
}

#[tokio::test]
async fn test_register_endpoint_missing_fields() {
    // Arrange
    let (app, _container) = create_test_app().await;

    let register_body = json!({
        "email": "test@example.com"
        // Missing password and full_name
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Missing fields should return 400"
    );
}

#[tokio::test]
async fn test_login_endpoint_success() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return; // Skip test in CI without database
    }

    // Arrange - First register the user
    let (app, _container) = create_test_app().await;
    let unique_email = format!("login_test_{}@example.com", uuid::Uuid::new_v4());

    // Register first
    let register_body = json!({
        "email": unique_email,
        "password": "SecureP@ss123!",
        "full_name": "Login Test User"
    });

    let _response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Now login
    let login_body = json!({
        "email": unique_email,
        "password": "SecureP@ss123!"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Login should return 200 OK"
    );
}

#[tokio::test]
async fn test_login_endpoint_invalid_credentials() {
    // This test can run without a database
    // Arrange
    let (app, _container) = create_test_app().await;

    let login_body = json!({
        "email": "nonexistent@example.com",
        "password": "SomePassword123!"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid credentials should return 401"
    );
}

#[tokio::test]
async fn test_login_endpoint_validation_error() {
    // Arrange
    let (app, _container) = create_test_app().await;

    let login_body = json!({
        "email": "invalid-email-format",
        "password": "password123"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Invalid login data should return 400"
    );
}

#[tokio::test]
async fn test_protected_endpoint_without_auth() {
    // Arrange
    let (app, _container) = create_test_app().await;

    // Act - Try to access protected route without auth
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users/profile")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Protected route without auth should return 401"
    );
}

#[tokio::test]
async fn test_protected_endpoint_invalid_token() {
    // Arrange
    let (app, _container) = create_test_app().await;

    // Act - Try to access protected route with invalid token
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users/profile")
                .header("authorization", "Bearer invalid_token_here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Protected route with invalid token should return 401"
    );
}

#[tokio::test]
async fn test_protected_endpoint_malformed_header() {
    // Arrange
    let (app, _container) = create_test_app().await;

    // Act - Try with malformed authorization header
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users/profile")
                .header("authorization", "NotBearer token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed auth header should return 401"
    );
}

#[tokio::test]
async fn test_refresh_endpoint_success() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return; // Skip test in CI without database
    }

    // Arrange - First register and login to get refresh token
    let (app, _container) = create_test_app().await;
    let unique_email = format!("refresh_test_{}@example.com", uuid::Uuid::new_v4());

    // Register
    let register_body = json!({
        "email": unique_email,
        "password": "SecureP@ss123!",
        "full_name": "Refresh Test User"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Extract refresh token from response
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let refresh_token = json["refresh_token"].as_str().unwrap();

    // Now refresh
    let refresh_body = json!({
        "refresh_token": refresh_token
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(refresh_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Token refresh should return 200 OK"
    );
}

#[tokio::test]
async fn test_refresh_endpoint_invalid_token() {
    // Arrange
    let (app, _container) = create_test_app().await;

    let refresh_body = json!({
        "refresh_token": "invalid_refresh_token_value"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(refresh_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid refresh token should return 401"
    );
}

#[tokio::test]
async fn test_health_endpoint() {
    // Arrange
    let (app, _container) = create_test_app().await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Health endpoint should return 200 OK"
    );
}

#[tokio::test]
async fn test_nonexistent_endpoint() {
    // Arrange
    let (app, _container) = create_test_app().await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Nonexistent endpoint should return 404"
    );
}

use std::sync::Arc;
