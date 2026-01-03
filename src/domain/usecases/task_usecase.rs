//! Task use cases implementation
//!
//! This module contains the business logic for task operations including
//! CRUD operations with proper authorization checks.

use crate::domain::entities::{CreateTask, CreateTaskRequest, Task, UpdateTaskRequest};
use crate::domain::repositories::TaskRepository;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Custom error types for task use case operations
#[derive(thiserror::Error, Debug)]
pub enum TaskUsecaseError {
    #[error("Task not found with id: {0}")]
    TaskNotFound(Uuid),

    #[error("Task not owned by user: {0}")]
    TaskNotOwned(Uuid),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid task status: {0}")]
    InvalidTaskStatus(String),

    #[error("Invalid task priority: {0}")]
    InvalidTaskPriority(String),
}

impl From<anyhow::Error> for TaskUsecaseError {
    fn from(e: anyhow::Error) -> Self {
        TaskUsecaseError::DatabaseError(e.to_string())
    }
}

/// Task use case trait
#[async_trait]
pub trait TaskUsecase: Send + Sync {
    /// Create a new task for a user
    async fn create_task(
        &self,
        user_id: Uuid,
        req: CreateTaskRequest,
    ) -> Result<Task, TaskUsecaseError>;

    /// Get a task by ID
    async fn get_task(&self, id: Uuid, user_id: Uuid) -> Result<Task, TaskUsecaseError>;

    /// List tasks for a user with pagination
    async fn list_tasks(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Task>, i64), TaskUsecaseError>;

    /// Update a task
    async fn update_task(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateTaskRequest,
    ) -> Result<Task, TaskUsecaseError>;

    /// Delete a task
    async fn delete_task(&self, id: Uuid, user_id: Uuid) -> Result<(), TaskUsecaseError>;
}

/// Task use case implementation
pub struct TaskUsecaseImpl {
    task_repo: Arc<dyn TaskRepository>,
}

impl TaskUsecaseImpl {
    /// Create a new TaskUsecaseImpl
    ///
    /// # Arguments
    /// * `task_repo` - Task repository implementation
    ///
    /// # Returns
    /// A new TaskUsecaseImpl instance
    pub fn new(task_repo: Arc<dyn TaskRepository>) -> Self {
        Self { task_repo }
    }

    /// Validate and normalize limit
    fn validate_limit(limit: i64) -> i64 {
        if limit <= 0 || limit > 100 {
            20 // Default limit
        } else {
            limit
        }
    }

    /// Validate and normalize offset
    fn validate_offset(offset: i64) -> i64 {
        if offset < 0 {
            0
        } else {
            offset
        }
    }
}

#[async_trait]
impl TaskUsecase for TaskUsecaseImpl {
    /// Create a new task for a user
    async fn create_task(
        &self,
        user_id: Uuid,
        req: CreateTaskRequest,
    ) -> Result<Task, TaskUsecaseError> {
        // Validate the request
        req.validate_request()
            .map_err(|e| TaskUsecaseError::ValidationError(e.to_string()))?;

        // Create the task
        let create_task = CreateTask::new(
            user_id,
            req.title.clone(),
            req.description.clone(),
            req.get_priority(),
            req.due_date,
        );

        let task = self
            .task_repo
            .create(&create_task)
            .await
            .context("Failed to create task")?;

        Ok(task)
    }

    /// Get a task by ID
    async fn get_task(&self, id: Uuid, user_id: Uuid) -> Result<Task, TaskUsecaseError> {
        let task = self
            .task_repo
            .find_by_id(id)
            .await
            .context("Failed to find task by id")?
            .ok_or(TaskUsecaseError::TaskNotFound(id))?;

        // Verify ownership
        if task.user_id != user_id {
            return Err(TaskUsecaseError::TaskNotOwned(id));
        }

        Ok(task)
    }

    /// List tasks for a user with pagination
    async fn list_tasks(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Task>, i64), TaskUsecaseError> {
        let limit = Self::validate_limit(limit);
        let offset = Self::validate_offset(offset);

        let (tasks, total) = self
            .task_repo
            .find_by_user_id(user_id, limit, offset)
            .await
            .context("Failed to find tasks by user id")?;

        Ok((tasks, total))
    }

    /// Update a task
    async fn update_task(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateTaskRequest,
    ) -> Result<Task, TaskUsecaseError> {
        // Validate the request if it has updates
        if req.has_updates() {
            req.validate_request()
                .map_err(|e| TaskUsecaseError::ValidationError(e.to_string()))?;
        }

        // Find existing task
        let mut task = self
            .task_repo
            .find_by_id(id)
            .await
            .context("Failed to find task by id")?
            .ok_or(TaskUsecaseError::TaskNotFound(id))?;

        // Verify ownership
        if task.user_id != user_id {
            return Err(TaskUsecaseError::TaskNotOwned(id));
        }

        // Apply updates
        if let Some(title) = &req.title {
            task.title = title.clone();
        }

        if let Some(description) = &req.description {
            task.description = Some(description.clone());
        }

        if let Some(status) = req.status {
            task.status = status;
            // Update completed_at if status is completed
            if status == crate::domain::entities::TaskStatus::Completed {
                task.completed_at = Some(Utc::now());
            } else {
                task.completed_at = None;
            }
        }

        if let Some(priority) = req.priority {
            task.priority = priority;
        }

        if let Some(due_date) = req.due_date {
            task.due_date = Some(due_date);
        }

        // Update in repository
        let updated_task = self
            .task_repo
            .update(&task)
            .await
            .context("Failed to update task")?;

        Ok(updated_task)
    }

    /// Delete a task
    async fn delete_task(&self, id: Uuid, user_id: Uuid) -> Result<(), TaskUsecaseError> {
        // Find existing task
        let task = self
            .task_repo
            .find_by_id(id)
            .await
            .context("Failed to find task by id")?
            .ok_or(TaskUsecaseError::TaskNotFound(id))?;

        // Verify ownership
        if task.user_id != user_id {
            return Err(TaskUsecaseError::TaskNotOwned(id));
        }

        self.task_repo
            .delete(id, user_id)
            .await
            .context("Failed to delete task")?;

        Ok(())
    }
}

/// List tasks response structure
#[derive(Debug)]
pub struct ListTasksResponse {
    pub tasks: Vec<Task>,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::TaskRepository;
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    // Mock task repository for testing
    struct MockTaskRepository {
        tasks: std::sync::Mutex<Vec<Task>>,
    }

    #[async_trait]
    impl TaskRepository for MockTaskRepository {
        async fn create(&self, task: &CreateTask) -> Result<Task> {
            let mut tasks = self.tasks.lock().unwrap();
            let id = Uuid::new_v4();
            let now = Utc::now();
            let new_task = Task {
                id,
                user_id: task.user_id,
                title: task.title.clone(),
                description: task.description.clone(),
                status: crate::domain::entities::TaskStatus::Pending,
                priority: task.priority,
                due_date: task.due_date,
                completed_at: None,
                created_at: now,
                updated_at: now,
            };
            tasks.push(new_task.clone());
            Ok(new_task)
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<Task>> {
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().find(|t| t.id == id).cloned())
        }

        async fn find_by_user_id(
            &self,
            user_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> Result<(Vec<Task>, i64)> {
            let tasks = self.tasks.lock().unwrap();
            let user_tasks: Vec<Task> = tasks
                .iter()
                .filter(|t| t.user_id == user_id)
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            let total = tasks.iter().filter(|t| t.user_id == user_id).count() as i64;
            Ok((user_tasks, total))
        }

        async fn update(&self, task: &Task) -> Result<Task> {
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(idx) = tasks.iter().position(|t| t.id == task.id) {
                tasks[idx] = task.clone();
                Ok(tasks[idx].clone())
            } else {
                Err(anyhow::anyhow!("Task not found"))
            }
        }

        async fn delete(&self, id: Uuid, _user_id: Uuid) -> Result<()> {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.retain(|t| t.id != id);
            Ok(())
        }

        async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64> {
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().filter(|t| t.user_id == user_id).count() as i64)
        }
    }

    #[tokio::test]
    async fn test_create_task_success() {
        let mock_repo = Arc::new(MockTaskRepository {
            tasks: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = TaskUsecaseImpl::new(mock_repo);

        let user_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Test Description".to_string()),
            priority: Some(crate::domain::entities::TaskPriority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        };

        let result = usecase.create_task(user_id, req).await;
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.user_id, user_id);
    }

    #[tokio::test]
    async fn test_get_task_success() {
        let mock_repo = Arc::new(MockTaskRepository {
            tasks: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        let created = usecase.create_task(user_id, req).await.unwrap();

        let result = usecase.get_task(created.id, user_id).await;
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.id, created.id);
    }

    #[tokio::test]
    async fn test_get_task_not_owned() {
        let mock_repo = Arc::new(MockTaskRepository {
            tasks: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        let created = usecase.create_task(user_id, req).await.unwrap();

        let result = usecase.get_task(created.id, other_user_id).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskUsecaseError::TaskNotOwned(_)
        ));
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let mock_repo = Arc::new(MockTaskRepository {
            tasks: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();

        // Create multiple tasks
        for i in 0..5 {
            let req = CreateTaskRequest {
                title: format!("Test Task {i}"),
                description: None,
                priority: None,
                due_date: None,
            };
            usecase.create_task(user_id, req).await.unwrap();
        }

        let result = usecase.list_tasks(user_id, 10, 0).await;
        assert!(result.is_ok());
        let (tasks, total) = result.unwrap();
        assert_eq!(tasks.len(), 5);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let mock_repo = Arc::new(MockTaskRepository {
            tasks: std::sync::Mutex::new(Vec::new()),
        });
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        let created = usecase.create_task(user_id, req).await.unwrap();

        let result = usecase.delete_task(created.id, user_id).await;
        assert!(result.is_ok());

        // Verify task is deleted
        let get_result = usecase.get_task(created.id, user_id).await;
        assert!(get_result.is_err());
        assert!(matches!(
            get_result.unwrap_err(),
            TaskUsecaseError::TaskNotFound(_)
        ));
    }
}
