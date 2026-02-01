use async_trait::async_trait;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::task::dto::{CreateTaskRequest, TaskListResponse, TaskResponse, UpdateTaskRequest};
use crate::internal::domain::task::entity::Task;

/// Repository trait for task data access operations
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Create a new task
    async fn create(&self, task: &Task) -> Result<(), DomainError>;

    /// Get task by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Task, DomainError>;

    /// Get task by ID and user ID (for authorization)
    async fn get_by_user_and_id(&self, user_id: Uuid, task_id: Uuid) -> Result<Task, DomainError>;

    /// List tasks for a user with pagination
    async fn list_by_user(
        &self,
        user_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> Result<(Vec<Task>, u64), DomainError>;

    /// Update task
    async fn update(&self, task: &Task) -> Result<(), DomainError>;

    /// Delete task by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Delete all tasks for a user (cascade)
    async fn delete_by_user(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Count tasks for a user
    async fn count_by_user(&self, user_id: Uuid) -> Result<u64, DomainError>;
}

/// Service trait for task business logic
#[async_trait]
pub trait TaskService: Send + Sync {
    /// Create a new task for a user
    async fn create_task(&self, user_id: Uuid, req: CreateTaskRequest) -> Result<TaskResponse, DomainError>;

    /// Get a task by ID for a user
    async fn get_task(&self, user_id: Uuid, task_id: Uuid) -> Result<TaskResponse, DomainError>;

    /// List tasks for a user with pagination
    async fn list_tasks(
        &self,
        user_id: Uuid,
        page: u64,
        per_page: u64,
    ) -> Result<TaskListResponse, DomainError>;

    /// Update a task for a user
    async fn update_task(
        &self,
        user_id: Uuid,
        task_id: Uuid,
        req: UpdateTaskRequest,
    ) -> Result<TaskResponse, DomainError>;

    /// Delete a task for a user
    async fn delete_task(&self, user_id: Uuid, task_id: Uuid) -> Result<(), DomainError>;
}
