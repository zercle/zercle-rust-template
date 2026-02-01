use std::sync::Arc;

use axum::{
    http::{Method, StatusCode},
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::internal::infrastructure::{
    di::Container,
    http::{
        handlers::{health, task, user},
        middleware::{auth, error, rate_limit, request_id},
    },
};

/// Create the application router with all routes and middleware
pub fn create_router(container: Arc<Container>) -> Router {
    // Create handlers with their dependencies
    let user_handler = Arc::new(user::UserHandler::new(container.user_service.clone()));
    let task_handler = Arc::new(task::TaskHandler::new(container.task_service.clone()));

    // Clone for middleware
    let jwt_generator = container.jwt_generator.clone();
    let rate_limiter = container.rate_limiter.clone();

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/api/v1/auth/register", post(user::UserHandler::register))
        .route("/api/v1/auth/login", post(user::UserHandler::login))
        .route("/api/v1/auth/refresh", post(user::UserHandler::refresh))
        .with_state(user_handler.clone());

    // Protected routes (auth required) - shared state for both handlers
    let protected_routes = Router::new()
        // User routes
        .route("/api/v1/auth/logout", post(user::UserHandler::logout))
        .route("/api/v1/users/profile", get(user::UserHandler::get_profile))
        .route("/api/v1/users/profile", put(user::UserHandler::update_profile))
        .route("/api/v1/users/profile", delete(user::UserHandler::delete_account))
        // Task routes
        .route("/api/v1/tasks", post(task::TaskHandler::create))
        .route("/api/v1/tasks", get(task::TaskHandler::list))
        .route("/api/v1/tasks/:id", get(task::TaskHandler::get_by_id))
        .route("/api/v1/tasks/:id", put(task::TaskHandler::update))
        .route("/api/v1/tasks/:id", delete(task::TaskHandler::delete))
        .with_state(user_handler)
        .with_state(task_handler)
        .layer(axum::middleware::from_fn(move |req, next| {
            let jwt = jwt_generator.clone();
            auth::auth_middleware(jwt, req, next)
        }));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // Add middleware layers (order matters - last added is first executed)
        .layer(axum::middleware::from_fn(error::error_handler_middleware))
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
        .layer(axum::middleware::from_fn(move |req, next| {
            let limiter = rate_limiter.clone();
            rate_limit::rate_limit_middleware(limiter, req, next)
        }))
        .layer(cors_layer())
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .on_request(tower_http::trace::DefaultOnRequest::new())
                .on_response(tower_http::trace::DefaultOnResponse::new()),
        )
}

/// Create CORS layer
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://localhost:5173".parse().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(false)
}
