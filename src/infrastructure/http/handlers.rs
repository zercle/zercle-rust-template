//! HTTP handlers module
//!
//! This module contains request handlers for User and Task endpoints.

use crate::domain::entities::{
    CreateTaskRequest, CreateUserRequest, LoginRequest, UpdateTaskRequest, UpdateUserRequest, User,
};
use crate::domain::usecases::{TaskUsecase, TaskUsecaseError, UserUsecase, UserUsecaseError};
use crate::infrastructure::db::connection::DbPool;
use crate::infrastructure::middleware::auth::UserId;
use axum::http::StatusCode;
use axum::{
    extract::{Extension, Path, Query},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Response Types
// ============================================================================

/// Generic API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    pub fn with_message(data: Option<T>, message: &str) -> Self {
        Self {
            success: true,
            data,
            message: Some(message.to_string()),
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(message.to_string()),
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            data,
            total,
            limit,
            offset,
        }
    }
}

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}
fn default_offset() -> i64 {
    0
}

// ============================================================================
// Auth Handlers
// ============================================================================

pub async fn register(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    match user_usecase.register(req).await {
        Ok(response) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(RegisterResponse {
                user: response.user,
                token: response.token,
            })),
        ),
        Err(UserUsecaseError::UserAlreadyExists(email)) => (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(&format!(
                "User already exists with email: {}",
                email
            ))),
        ),
        Err(UserUsecaseError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(&msg)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: User,
    pub token: String,
}

pub async fn login(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match user_usecase.login(req).await {
        Ok(response) => (
            StatusCode::OK,
            Json(ApiResponse::success(LoginResponse {
                user: response.user,
                token: response.token,
            })),
        ),
        Err(UserUsecaseError::InvalidCredentials(msg)) => {
            (StatusCode::UNAUTHORIZED, Json(ApiResponse::error(&msg)))
        }
        Err(UserUsecaseError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(&msg)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
}

// ============================================================================
// User Handlers
// ============================================================================

pub async fn get_profile(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    match user_usecase.get_profile(user_id.0).await {
        Ok(user) => (StatusCode::OK, Json(ApiResponse::success(user))),
        Err(UserUsecaseError::UserNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn update_profile(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    match user_usecase.update_profile(user_id.0, req).await {
        Ok(user) => (StatusCode::OK, Json(ApiResponse::success(user))),
        Err(UserUsecaseError::UserNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found")),
        ),
        Err(UserUsecaseError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(&msg)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn delete_account(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    match user_usecase.delete_account(user_id.0).await {
        Ok(_) => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<()>::with_message(
                None,
                "Account deleted successfully",
            )),
        ),
        Err(UserUsecaseError::UserNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn list_users(
    Extension(user_usecase): Extension<Arc<dyn UserUsecase>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = if params.limit <= 0 || params.limit > 100 {
        20
    } else {
        params.limit
    };
    let offset = if params.offset < 0 { 0 } else { params.offset };

    match user_usecase.list_users(limit, offset).await {
        Ok((users, total)) => (
            StatusCode::OK,
            Json(ApiResponse::success(PaginatedResponse::new(
                users, total, limit, offset,
            ))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

// ============================================================================
// Task Handlers
// ============================================================================

pub async fn create_task(
    Extension(task_usecase): Extension<Arc<dyn TaskUsecase>>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    match task_usecase.create_task(user_id.0, req).await {
        Ok(task) => (StatusCode::CREATED, Json(ApiResponse::success(task))),
        Err(TaskUsecaseError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(&msg)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn get_task(
    Extension(task_usecase): Extension<Arc<dyn TaskUsecase>>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match task_usecase.get_task(id, user_id.0).await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse::success(task))),
        Err(TaskUsecaseError::TaskNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Task not found")),
        ),
        Err(TaskUsecaseError::TaskNotOwned(_)) => (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "You don't have permission to access this task",
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn list_tasks(
    Extension(task_usecase): Extension<Arc<dyn TaskUsecase>>,
    Extension(user_id): Extension<UserId>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = if params.limit <= 0 || params.limit > 100 {
        20
    } else {
        params.limit
    };
    let offset = if params.offset < 0 { 0 } else { params.offset };

    match task_usecase.list_tasks(user_id.0, limit, offset).await {
        Ok((tasks, total)) => (
            StatusCode::OK,
            Json(ApiResponse::success(PaginatedResponse::new(
                tasks, total, limit, offset,
            ))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn update_task(
    Extension(task_usecase): Extension<Arc<dyn TaskUsecase>>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    match task_usecase.update_task(id, user_id.0, req).await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse::success(task))),
        Err(TaskUsecaseError::TaskNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Task not found")),
        ),
        Err(TaskUsecaseError::TaskNotOwned(_)) => (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "You don't have permission to update this task",
            )),
        ),
        Err(TaskUsecaseError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(&msg)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

pub async fn delete_task(
    Extension(task_usecase): Extension<Arc<dyn TaskUsecase>>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match task_usecase.delete_task(id, user_id.0).await {
        Ok(_) => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<()>::with_message(
                None,
                "Task deleted successfully",
            )),
        ),
        Err(TaskUsecaseError::TaskNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Task not found")),
        ),
        Err(TaskUsecaseError::TaskNotOwned(_)) => (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "You don't have permission to delete this task",
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        ),
    }
}

// ============================================================================
// Health Handlers
// ============================================================================

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::<()>::with_message(None, "Service is healthy")),
    )
}

pub async fn readiness_check(Extension(db): Extension<DbPool>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").fetch_one(&db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::<()>::with_message(None, "Service is ready")),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::error("Database connection is not available")),
        ),
    }
}
