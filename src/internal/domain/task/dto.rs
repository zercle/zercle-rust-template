use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request DTO for creating a new task
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: String,

    #[validate(length(max = 2000, message = "Description must be less than 2000 characters"))]
    pub description: Option<String>,

    /// Status: pending, in_progress, completed, cancelled
    #[serde(default)]
    pub status: Option<String>,

    /// Priority: low, medium, high, urgent
    #[serde(default)]
    pub priority: Option<String>,

    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// Request DTO for updating an existing task
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskRequest {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: Option<String>,

    #[validate(length(max = 2000, message = "Description must be less than 2000 characters"))]
    pub description: Option<String>,

    /// Status: pending, in_progress, completed, cancelled
    #[serde(default)]
    pub status: Option<String>,

    /// Priority: low, medium, high, urgent
    #[serde(default)]
    pub priority: Option<String>,

    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// Response DTO for task data
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response DTO for paginated task list
#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

/// Validates task status string
pub fn validate_task_status(status: &str) -> bool {
    matches!(status, "pending" | "in_progress" | "completed" | "cancelled")
}

/// Validates task priority string
pub fn validate_task_priority(priority: &str) -> bool {
    matches!(priority, "low" | "medium" | "high" | "urgent")
}
