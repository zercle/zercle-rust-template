//! Integration tests for API endpoints
//!
//! These tests verify the API endpoints with a real database connection.
//! They test the full request/response cycle including authentication.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Duration, Utc};
use reqwest;

use zercle_rust_template::config::Settings;
use zercle_rust_template::infrastructure::db::connection::Database;
use zercle_rust_template::infrastructure::db::migrations::Migrations;
use zercle_rust_template::infrastructure::http::routes::create_router;
use zercle_rust_template::infrastructure::http::server::Server;
use zercle_rust_template::domain::usecases::{TaskUsecaseImpl, UserUsecaseImpl};
use zercle_rust_template::domain::entities::{CreateUserRequest, LoginRequest, CreateTaskRequest, TaskPriority};
use zercle_rust_template::infrastructure::db::postgres_repository::{PostgresUserRepository, PostgresTaskRepository};
use std::sync::Arc;

/// Test configuration for integration tests
struct TestConfig {
    pub pool: Pool<Postgres>,
    pub base_url: String,
    pub settings: Settings,
}

/// Setup test database and application
async fn setup_test_app() -> TestConfig {
    let settings = Settings {
        server: Settings::from_env().unwrap().server,
        database: Settings::from_env().unwrap().database,
        jwt: Settings::from_env().unwrap().jwt,
        logging: Settings::from_env().unwrap().logging,
        cors: Settings::from_env().unwrap().cors,
        rate_limit: Settings::from_env().unwrap().rate_limit,
        argon2id: Settings::from_env().unwrap().argon2id,
    };

    // Connect to database
    let db = Database::connect(&settings).await.expect("Failed to connect to database");
    
    // Run migrations
    Migrations::run(db.pool()).await.expect("Failed to run migrations");

    TestConfig {
        pool: db.pool().clone(),
        base_url: format!("http://localhost:{}", settings.server.port),
        settings,
    }
}

/// Helper function to register a test user
async fn register_test_user(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> reqwest::Result<String> {
    let response = client
        .post(&format!("{}/api/v1/auth/register", base_url))
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

/// Helper function to login and get token
async fn login_test_user(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> reqwest::Result<String> {
    let response = client
        .post(&format!("{}/api/v1/auth/login", base_url))
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    Ok(body["data"]["token"].as_str().unwrap_or("").to_string())
}

mod auth_tests {
    use super::*;

    /// Test user registration
    #[sqlx::test]
    async fn test_register_and_login(pool: Pool<Postgres>) {
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        // Start a test server
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();
        let email = format!("test_{}@example.com", Uuid::new_v4());

        // Test registration
        let response = client
            .post(&format!("{}/api/v1/auth/register", base_url))
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
        let token = body["data"]["token"].as_str().unwrap().to_string();

        // Test login with same credentials
        let response = client
            .post(&format!("{}/api/v1/auth/login", base_url))
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
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();
        let email = format!("duplicate_{}@example.com", Uuid::new_v4());

        // First registration
        let response = client
            .post(&format!("{}/api/v1/auth/register", base_url))
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
            .post(&format!("{}/api/v1/auth/register", base_url))
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
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();
        let email = format!("task_user_{}@example.com", Uuid::new_v4());

        // Register and login
        let token = register_test_user(&client, &base_url, &email, "Password123!").await;

        // Create a task
        let response = client
            .post(&format!("{}/api/v1/tasks", base_url))
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
            .get(&format!("{}/api/v1/tasks/{}", base_url, task_id))
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
            .put(&format!("{}/api/v1/tasks/{}", base_url, task_id))
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
            .get(&format!("{}/api/v1/tasks", base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let list_body: serde_json::Value = response.json().await.unwrap();
        assert!(list_body["data"]["data"].is_array());

        // Delete the task
        let response = client
            .delete(&format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 204);

        // Verify task is deleted
        let response = client
            .get(&format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    }

    /// Test unauthorized access
    #[sqlx::test]
    async fn test_unauthorized_access(pool: Pool<Postgres>) {
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();

        // Try to access protected endpoint without token
        let response = client
            .get(&format!("{}/api/v1/tasks", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);

        // Try with invalid token
        let response = client
            .get(&format!("{}/api/v1/tasks", base_url))
            .header("Authorization", "Bearer invalid_token")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);
    }

    /// Test task ownership
    #[sqlx::test]
    async fn test_task_ownership(pool: Pool<Postgres>) {
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();

        // Create two users
        let email1 = format!("user1_{}@example.com", Uuid::new_v4());
        let email2 = format!("user2_{}@example.com", Uuid::new_v4());

        let token1 = register_test_user(&client, &base_url, &email1, "Password123!").await;
        let token2 = register_test_user(&client, &base_url, &email2, "Password123!").await;

        // User 1 creates a task
        let response = client
            .post(&format!("{}/api/v1/tasks", base_url))
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
            .get(&format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token2))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 403);

        // User 2 tries to delete User 1's task
        let response = client
            .delete(&format!("{}/api/v1/tasks/{}", base_url, task_id))
            .header("Authorization", format!("Bearer {}", token2))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 403);

        // User 1 can access their own task
        let response = client
            .get(&format!("{}/api/v1/tasks/{}", base_url, task_id))
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
        let settings = Settings::from_env().unwrap();
        let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
        let task_repo = Arc::new(PostgresTaskRepository::new(pool.clone()));
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));

        let router = create_router(user_usecase, task_usecase, &settings, pool.clone());
        
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router);
        let addr = server.local_addr();
        let base_url = format!("http://{}", addr);
        
        tokio::spawn(async move {
            let _ = server.await;
        });

        let client = reqwest::Client::new();

        // Test health endpoint (no auth required)
        let response = client
            .get(&format!("{}/health", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["message"], "Service is healthy");

        // Test readiness endpoint (requires DB connection)
        let response = client
            .get(&format!("{}/readiness", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
    }
}
