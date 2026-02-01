//! Integration tests for task endpoints
//!
//! These tests verify the HTTP API layer for task CRUD operations
//! including creating, listing, updating, and deleting tasks.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use zercle_rust_template::internal::infrastructure::{
    di::Container,
    http::create_router,
};

use crate::integration::setup_test_container;

/// Create a test app with test configuration
async fn create_test_app() -> (axum::Router, Arc<Container>, String) {
    let container = setup_test_container().await;
    let app = create_router(container.clone());

    // Generate unique email for this test run
    let unique_email = format!("task_test_{}@example.com", Uuid::new_v4());

    (app, container, unique_email)
}

/// Helper to register a user and return access token
async fn register_and_get_token(app: &axum::Router, email: &str) -> String {
    let register_body = json!({
        "email": email,
        "password": "SecureP@ss123!",
        "full_name": "Task Test User"
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

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    json["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_create_task_endpoint() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return; // Skip test in CI without database
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;

    let task_body = json!({
        "title": "Integration Test Task",
        "description": "This is a test task created during integration testing",
        "status": "pending",
        "priority": "high"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("authorization", format!("Bearer {}", access_token))
                .header("content-type", "application/json")
                .body(Body::from(task_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Create task should return 201 Created"
    );
}

#[tokio::test]
async fn test_create_task_with_minimal_data() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;

    let task_body = json!({
        "title": "Minimal Task"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("authorization", format!("Bearer {}", access_token))
                .header("content-type", "application/json")
                .body(Body::from(task_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Create task with minimal data should succeed"
    );
}

#[tokio::test]
async fn test_create_task_without_auth() {
    // Arrange
    let (app, _container, _) = create_test_app().await;

    let task_body = json!({
        "title": "Unauthorized Task"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("content-type", "application/json")
                .body(Body::from(task_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Create task without auth should return 401"
    );
}

#[tokio::test]
async fn test_create_task_validation_error() {
    // This test can run without database (validation only)
    // Arrange
    let (app, _container, _) = create_test_app().await;

    let task_body = json!({
        "title": "" // Empty title should fail validation
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("authorization", "Bearer invalid_token")
                .header("content-type", "application/json")
                .body(Body::from(task_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should fail auth before validation"
    );
}

#[tokio::test]
async fn test_list_tasks_endpoint() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .header("authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "List tasks should return 200 OK"
    );

    // Check response structure
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(
        json.get("tasks").is_some(),
        "Response should contain tasks array"
    );
    assert!(
        json.get("total").is_some(),
        "Response should contain total count"
    );
    assert!(
        json.get("page").is_some(),
        "Response should contain page number"
    );
    assert!(
        json.get("per_page").is_some(),
        "Response should contain per_page"
    );
}

#[tokio::test]
async fn test_list_tasks_pagination() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;

    // Act - Request second page with 5 items per page
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?page=2&per_page=5")
                .header("authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "List tasks with pagination should return 200"
    );
}

#[tokio::test]
async fn test_list_tasks_without_auth() {
    // Arrange
    let (app, _container, _) = create_test_app().await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "List tasks without auth should return 401"
    );
}

#[tokio::test]
async fn test_get_task_by_id_endpoint() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;
    let task_id = "00000000-0000-0000-0000-000000000001"; // Use a valid UUID format

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header("authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert - Either 200 (found) or 404 (not found) are valid for this test
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Get task should return 200 or 404"
    );
}

#[tokio::test]
async fn test_get_nonexistent_task() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;
    let nonexistent_id = Uuid::new_v4();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{}", nonexistent_id))
                .header("authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Get nonexistent task should return 404"
    );
}

#[tokio::test]
async fn test_update_task_endpoint() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;
    let task_id = "00000000-0000-0000-0000-000000000001";

    let update_body = json!({
        "title": "Updated Task Title",
        "status": "completed"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header("authorization", format!("Bearer {}", access_token))
                .header("content-type", "application/json")
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert - Either 200 (success) or 404 (not found) are valid
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Update task should return 200 or 404"
    );
}

#[tokio::test]
async fn test_update_task_partial() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;
    let task_id = "00000000-0000-0000-0000-000000000001";

    // Only update priority, leave other fields unchanged
    let update_body = json!({
        "priority": "urgent"
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header("authorization", format!("Bearer {}", access_token))
                .header("content-type", "application/json")
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Partial update should work"
    );
}

#[tokio::test]
async fn test_delete_task_endpoint() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange
    let (app, _container, unique_email) = create_test_app().await;
    let access_token = register_and_get_token(&app, &unique_email).await;
    let task_id = "00000000-0000-0000-0000-000000000001";

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header("authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert - Either 204 (success) or 404 (not found) are valid
    assert!(
        response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::NOT_FOUND,
        "Delete task should return 204 or 404"
    );
}

#[tokio::test]
async fn test_delete_task_without_auth() {
    // Arrange
    let (app, _container, _) = create_test_app().await;
    let task_id = Uuid::new_v4();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Delete task without auth should return 401"
    );
}

#[tokio::test]
async fn test_access_other_users_task() {
    // This test requires a running database
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    // Arrange - Create two users
    let (app, _container, email1) = create_test_app().await;
    let access_token1 = register_and_get_token(&app, &email1).await;

    // Create second user and get their task
    let (_, _, email2) = create_test_app().await;
    let access_token2 = register_and_get_token(&app, &email2).await;

    // Create a task with user 1
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("authorization", format!("Bearer {}", access_token1))
                .header("content-type", "application/json")
                .body(Body::from(json!({"title": "User 1 Task"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to access user 1's task with user 2's token
    // This should fail with 404 (not found) or 403 (forbidden)
    // depending on implementation
    let body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    if let Some(task_id) = json.get("id").and_then(|id| id.as_str()) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/tasks/{}", task_id))
                    .header("authorization", format!("Bearer {}", access_token2))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert - User 2 should not access User 1's task
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "User should not access other user's tasks"
        );
    }
}

use std::sync::Arc;
