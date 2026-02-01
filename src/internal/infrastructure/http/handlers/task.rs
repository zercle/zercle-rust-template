use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::internal::{
    domain::error::DomainError,
    domain::task::{
        dto::{CreateTaskRequest, UpdateTaskRequest},
        traits::TaskService,
    },
    infrastructure::http::response::success_empty,
    infrastructure::http::middleware::auth::AuthContext,
};

/// Query parameters for listing tasks
#[derive(Debug, Deserialize, Default)]
pub struct ListTasksQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

/// Task HTTP handler
pub struct TaskHandler {
    service: Arc<dyn TaskService>,
}

impl TaskHandler {
    /// Create a new TaskHandler
    pub fn new(service: Arc<dyn TaskService>) -> Self {
        Self { service }
    }

    /// POST /api/v1/tasks - Create a new task
    pub async fn create(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Json(req): Json<CreateTaskRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.create_task(auth.user_id, req).await?;
        Ok((StatusCode::CREATED, Json(json!({
            "status": "success",
            "data": response
        }))))
    }

    /// GET /api/v1/tasks - List tasks for authenticated user
    pub async fn list(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Query(params): Query<ListTasksQuery>,
    ) -> Result<impl IntoResponse, DomainError> {
        let page = params.page.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
        
        let response = handler.service.list_tasks(auth.user_id, page, per_page).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// GET /api/v1/tasks/:id - Get task by ID
    pub async fn get_by_id(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.get_task(auth.user_id, id).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// PUT /api/v1/tasks/:id - Update a task
    pub async fn update(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Path(id): Path<Uuid>,
        Json(req): Json<UpdateTaskRequest>,
    ) -> Result<impl IntoResponse, DomainError> {
        let response = handler.service.update_task(auth.user_id, id, req).await?;
        Ok(Json(json!({
            "status": "success",
            "data": response
        })))
    }

    /// DELETE /api/v1/tasks/:id - Delete a task
    pub async fn delete(
        State(handler): State<Arc<Self>>,
        auth: Extension<AuthContext>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse, DomainError> {
        handler.service.delete_task(auth.user_id, id).await?;
        Ok((StatusCode::NO_CONTENT, success_empty()))
    }

    /// Create router for task routes
    pub fn routes(self: Arc<Self>) -> axum::Router {
        axum::Router::new()
            .route("/api/v1/tasks", axum::routing::post(Self::create))
            .route("/api/v1/tasks", axum::routing::get(Self::list))
            .route("/api/v1/tasks/:id", axum::routing::get(Self::get_by_id))
            .route("/api/v1/tasks/:id", axum::routing::put(Self::update))
            .route("/api/v1/tasks/:id", axum::routing::delete(Self::delete))
            .with_state(self)
    }
}
