use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::internal::{
    domain::error::DomainError,
    domain::user::{
        dto::{
            LoginRequest, RefreshRequest,
            RegisterRequest, UpdateProfileRequest,
        },
        traits::UserService,
    },
    infrastructure::http::response::success_empty,
    infrastructure::http::middleware::auth::AuthContext,
};

/// Request DTO for logout (local definition since not in domain dto)
#[derive(Debug, serde::Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// User HTTP handler
pub struct UserHandler {
    service: Arc<dyn UserService>,
}

impl UserHandler {
    /// Create a new UserHandler
    pub fn new(service: Arc<dyn UserService>) -> Self {
        Self { service }
    }

    /// POST /api/v1/auth/register - Register a new user
    pub async fn register(
        State(handler): State<Arc<Self>>,
        Json(req): Json<RegisterRequest>,
    ) -> Result<(StatusCode, impl IntoResponse), DomainError> {
        let response = handler.service.register(req).await?;
        Ok((StatusCode::CREATED, Json(json!({
            "status": "success",
            "data": response
        }))))
    }

    /// POST /api/v1/auth/login - Login user
    pub async fn login(
        State(handler): State<Arc<Self>>,
        Json(req): Json<LoginRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.login(req).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// POST /api/v1/auth/refresh - Refresh access token
    pub async fn refresh(
        State(handler): State<Arc<Self>>,
        Json(req): Json<RefreshRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.refresh(req).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// POST /api/v1/auth/logout - Logout user
    pub async fn logout(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Json(req): Json<LogoutRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        handler.service.logout(auth.user_id, req.refresh_token).await?;
        Ok(success_empty())
    }

    /// GET /api/v1/users/profile - Get user profile
    pub async fn get_profile(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.get_profile(auth.user_id).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// PUT /api/v1/users/profile - Update user profile
    pub async fn update_profile(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Json(req): Json<UpdateProfileRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.update_profile(auth.user_id, req).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// DELETE /api/v1/users/profile - Delete user account
    pub async fn delete_account(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
    ) -> Result<impl IntoResponse, DomainError> {
        handler.service.delete_account(auth.user_id).await?;
        Ok((StatusCode::NO_CONTENT, success_empty()))
    }

    /// Create router for user routes
    pub fn routes(self: Arc<Self>) -> axum::Router {
        axum::Router::new()
            .route("/api/v1/auth/register", axum::routing::post(Self::register))
            .route("/api/v1/auth/login", axum::routing::post(Self::login))
            .route("/api/v1/auth/refresh", axum::routing::post(Self::refresh))
            .route("/api/v1/auth/logout", axum::routing::post(Self::logout))
            .route("/api/v1/users/profile", axum::routing::get(Self::get_profile))
            .route("/api/v1/users/profile", axum::routing::put(Self::update_profile))
            .route(
                "/api/v1/users/profile",
                axum::routing::delete(Self::delete_account),
            )
            .with_state(self)
    }
}

use axum::extract::Extension;
