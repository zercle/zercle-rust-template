//! HTTP routes module
//!
//! This module defines all API routes for the application.

use crate::config::Settings;
use crate::domain::usecases::{TaskUsecase, UserUsecase};
use crate::infrastructure::db::connection::DbPool;
use crate::infrastructure::http::handlers;
use crate::infrastructure::middleware::auth::{auth_middleware, AuthState};
use crate::infrastructure::middleware::cors::create_cors_layer;
use crate::infrastructure::middleware::logging::create_logging_layer;
use crate::infrastructure::middleware::rate_limit::{rate_limit_middleware, RateLimitLayer};
use axum::{routing::{get, post, put, delete}, Extension, Router};
use std::sync::Arc;

/// Create the application router with all routes and middleware
pub fn create_router(
    user_usecase: Arc<dyn UserUsecase>,
    task_usecase: Arc<dyn TaskUsecase>,
    settings: &Settings,
    db: DbPool,
) -> Router {
    // Create middleware layers
    let cors_layer = create_cors_layer(settings);
    let logging_layer = create_logging_layer(settings);
    let rate_limit_layer = RateLimitLayer::new(settings);
    let auth_state = AuthState::new(settings);
    let rate_limit_state = rate_limit_layer.state().clone();

    // Build router with routes
    let router = Router::new()
        // Health check endpoints (public)
        .route("/health", get(handlers::health_check))
        .route("/readiness", get(handlers::readiness_check))
        // Auth routes (public)
        .route("/api/v1/auth/register", post(handlers::register))
        .route("/api/v1/auth/login", post(handlers::login))
        // User routes (authenticated)
        .route("/api/v1/users/profile", get(handlers::get_profile))
        .route("/api/v1/users/profile", put(handlers::update_profile))
        .route("/api/v1/users/profile", delete(handlers::delete_account))
        .route("/api/v1/users", get(handlers::list_users))
        // Task routes (authenticated)
        .route("/api/v1/tasks", post(handlers::create_task))
        .route("/api/v1/tasks", get(handlers::list_tasks))
        .route("/api/v1/tasks/:id", get(handlers::get_task))
        .route("/api/v1/tasks/:id", put(handlers::update_task))
        .route("/api/v1/tasks/:id", delete(handlers::delete_task))
        // Add shared state via extensions
        .layer(Extension(user_usecase))
        .layer(Extension(task_usecase))
        .layer(Extension(db));

    // Apply middleware layers in order: rate_limit -> cors -> logging -> auth
    router
        .layer(axum::middleware::from_fn(move |req, next| {
            rate_limit_middleware(rate_limit_state.clone(), req, next)
        }))
        .layer(cors_layer)
        .layer(logging_layer)
        .layer(axum::middleware::from_fn_with_state(auth_state, auth_middleware))
}
