//! Integration tests for API endpoints
//!
//! These tests verify the API endpoints with a real database connection.
//! They test the full request/response cycle including authentication.

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use std::net::SocketAddr;
use std::sync::Arc;
use zercle_rust_template::config::Settings;
use zercle_rust_template::domain::usecases::{TaskUsecaseImpl, UserUsecaseImpl};
use zercle_rust_template::infrastructure::db::migrations::Migrations;
use zercle_rust_template::infrastructure::db::postgres_repository::{
    PostgresTaskRepository, PostgresUserRepository,
};
use zercle_rust_template::infrastructure::http::routes::create_router;

/// Run database migrations for tests
async fn run_migrations(pool: &Pool<Postgres>) {
    Migrations::run(pool)
        .await
        .expect("Failed to run migrations");
}

/// Helper function to register a test user
async fn register_test_user(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> reqwest::Result<String> {
    let response = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "full_name": "Test User",
            "phone": "+1234567890"
        }))
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    Ok(body["data"]["token"].as_str().unwrap_or("").to_string())
}

/// Starts a test server and returns the base URL
async fn start_test_server(router: axum::Router) -> (String, tokio::task::JoinHandle<()>) {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let server = axum::serve(listener, router);
    let handle = tokio::spawn(async move {
        let _ = server.await;
    });

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (base_url, handle)
}

mod auth_tests {
    use super::*;

    /// Test user registration
    #[sqlx::test]
    async fn test_register_and_login(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();
        let email = format!("test_{}@example.com", Uuid::new_v4());

        // Test registration
        let response = client
            .post(format!("{}/api/v1/auth/register", base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": "Password123!",
                "full_name": "Test User",
                "phone": "+1234567890"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["data"]["token"].is_string());
        let _token = body["data"]["token"].as_str().unwrap().to_string();

        // Test login with same credentials
        let response = client
            .post(format!("{}/api/v1/auth/login", base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": "Password123!"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["data"]["user"]["email"], email);
    }

    /// Test duplicate registration fails
    #[sqlx::test]
    async fn test_duplicate_registration_fails(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();
        let email = format!("duplicate_{}@example.com", Uuid::new_v4());

        // First registration
        let response = client
            .post(format!("{}/api/v1/auth/register", base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": "Password123!",
                "full_name": "Test User"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);

        // Second registration with same email should fail
        let response = client
            .post(format!("{}/api/v1/auth/register", base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": "Password123!",
                "full_name": "Another User"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 409);
    }
}

mod task_tests {
    use super::*;

    /// Test task CRUD operations
    #[sqlx::test]
    async fn test_create_and_manage_task(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();
        let email = format!("task_user_{}@example.com", Uuid::new_v4());

        // Register and login
        let token = register_test_user(&client, &base_url, &email, "Password123!")
            .await
            .unwrap();

        // Create a task
        let response = client
            .post(format!("{}/api/v1/tasks", base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "title": "Test Task",
                "description": "Test Description",
                "priority": "high"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let create_body: serde_json::Value = response.json().await.unwrap();
        let task_id = create_body["data"]["id"].as_str().unwrap().to_string();

        // Get the task
        let response = client
            .get(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let get_body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(get_body["data"]["title"], "Test Task");
        assert_eq!(get_body["data"]["priority"], "high");

        // Update the task
        let response = client
            .put(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "title": "Updated Task",
                "status": "completed"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let update_body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(update_body["data"]["title"], "Updated Task");
        assert_eq!(update_body["data"]["status"], "completed");

        // List tasks
        let response = client
            .get(format!("{}/api/v1/tasks", base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let list_body: serde_json::Value = response.json().await.unwrap();
        assert!(list_body["data"]["data"].is_array());

        // Delete the task
        let response = client
            .delete(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 204);

        // Verify task is deleted
        let response = client
            .get(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    /// Test unauthorized access
    #[sqlx::test]
    async fn test_unauthorized_access(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();

        // Try to access protected endpoint without token
        let response = client
            .get(format!("{}/api/v1/tasks", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);

        // Try with invalid token
        let response = client
            .get(format!("{}/api/v1/tasks", base_url))
            .header("Authorization", "Bearer invalid_token")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);
    }

    /// Test task ownership
    #[sqlx::test]
    async fn test_task_ownership(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();

        // Create two users
        let email1 = format!("user1_{}@example.com", Uuid::new_v4());
        let email2 = format!("user2_{}@example.com", Uuid::new_v4());

        let token1 = register_test_user(&client, &base_url, &email1, "Password123!")
            .await
            .unwrap();
        let token2 = register_test_user(&client, &base_url, &email2, "Password123!")
            .await
            .unwrap();

        // User 1 creates a task
        let response = client
            .post(format!("{}/api/v1/tasks", base_url))
            .header("Authorization", format!("Bearer {}", token1))
            .json(&serde_json::json!({
                "title": "User 1's Task"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let body: serde_json::Value = response.json().await.unwrap();
        let task_id = body["data"]["id"].as_str().unwrap().to_string();

        // User 2 tries to access User 1's task
        let response = client
            .get(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token2))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 403);

        // User 2 tries to delete User 1's task
        let response = client
            .delete(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token2))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 403);

        // User 1 can access their own task
        let response = client
            .get(format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token1))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}

mod health_tests {
    use super::*;

    /// Test health check endpoint
    #[sqlx::test]
    async fn test_health_check(pool: Pool<Postgres>) {
        run_migrations(&pool).await;
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());

        let (base_url, _handle) = start_test_server(router).await;

        let client = reqwest::Client::new();

        // Test health endpoint (no auth required)
        let response = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["message"], "Service is healthy");

        // Test readiness endpoint (requires DB connection)
        let response = client
            .get(format!("{}/readiness", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
    }
}
