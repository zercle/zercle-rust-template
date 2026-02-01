use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::task::dto::{
    CreateTaskRequest, TaskListResponse, TaskResponse, UpdateTaskRequest,
};
use crate::internal::domain::task::entity::{Task, TaskPriority, TaskStatus};
use crate::internal::domain::task::traits::TaskRepository;
use crate::internal::domain::task::traits::TaskService;

/// Task service implementation with all business logic
pub struct TaskServiceImpl {
    task_repo: Arc<dyn TaskRepository>,
}

impl TaskServiceImpl {
    /// Create a new TaskServiceImpl
    #[allow(dead_code)]
    pub fn new(task_repo: Arc<dyn TaskRepository>) -> Self {
        Self { task_repo }
    }

    /// Convert Task entity to TaskResponse DTO
    fn task_to_response(task: &Task) -> TaskResponse {
        TaskResponse {
            id: task.id,
            user_id: task.user_id,
            title: task.title.clone(),
            description: task.description.clone(),
            status: task.status.to_string(),
            priority: task.priority.to_string(),
            due_date: task.due_date,
            completed_at: task.completed_at,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }

    /// Parse task status from string, default to Pending
    fn parse_status(status: &str) -> TaskStatus {
        match status {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }

    /// Parse task priority from string, default to Medium
    fn parse_priority(priority: &str) -> TaskPriority {
        match priority {
            "low" => TaskPriority::Low,
            "high" => TaskPriority::High,
            "urgent" => TaskPriority::Urgent,
            _ => TaskPriority::Medium,
        }
    }
}

#[async_trait]
impl TaskService for TaskServiceImpl {
    /// Create a new task for a user
    async fn create_task(
        &self,
        user_id: Uuid,
        req: CreateTaskRequest,
    ) -> Result<TaskResponse, DomainError> {
        // Create new Task entity
        let task = Task::new(
            Uuid::new_v4(),
            user_id,
            req.title.clone(),
            req.description,
            req.status.as_deref().map_or(TaskStatus::Pending, Self::parse_status),
            req.priority.as_deref().map_or(TaskPriority::Medium, Self::parse_priority),
            req.due_date,
        );

        // Save to repository
        self.task_repo.create(&task).await?;

        // Return TaskResponse
        Ok(Self::task_to_response(&task))
    }

    /// Get a task by ID for a user
    async fn get_task(&self, user_id: Uuid, task_id: Uuid) -> Result<TaskResponse, DomainError> {
        // Get task by ID and user_id (authorization check)
        let task = self.task_repo.get_by_user_and_id(user_id, task_id).await?;
        Ok(Self::task_to_response(&task))
    }

    /// List tasks for a user with pagination
    async fn list_tasks(
        &self,
        user_id: Uuid,
        page: u64,
        per_page: u64,
    ) -> Result<TaskListResponse, DomainError> {
        // Calculate offset
        let offset = (page - 1) * per_page;

        // Get tasks from repository with pagination
        let (tasks, total) = self.task_repo.list_by_user(user_id, offset, per_page).await?;

        // Convert to responses
        let task_responses = tasks.into_iter().map(|t| Self::task_to_response(&t)).collect();

        // Return TaskListResponse
        Ok(TaskListResponse {
            tasks: task_responses,
            total,
            page,
            per_page,
        })
    }

    /// Update a task for a user
    async fn update_task(
        &self,
        user_id: Uuid,
        task_id: Uuid,
        req: UpdateTaskRequest,
    ) -> Result<TaskResponse, DomainError> {
        // Get existing task
        let mut task = self.task_repo.get_by_user_and_id(user_id, task_id).await?;

        // Update fields using provided values
        if let Some(title) = &req.title {
            task.title = title.clone();
        }

        if let Some(description) = &req.description {
            task.description = Some(description.clone());
        }

        if let Some(priority) = &req.priority {
            task.priority = Self::parse_priority(priority);
        }

        if let Some(due_date) = req.due_date {
            task.due_date = Some(due_date);
        }

        // Handle status changes
        if let Some(status_str) = &req.status {
            let new_status = Self::parse_status(status_str);

            // If status changed to Completed, set completed_at
            if new_status == TaskStatus::Completed && task.status != TaskStatus::Completed {
                task.status = new_status;
                task.completed_at = Some(Utc::now());
            }
            // If status changed from Completed, clear completed_at
            else if task.status == TaskStatus::Completed && new_status != TaskStatus::Completed {
                task.status = new_status;
                task.completed_at = None;
            }
            // Status changed but not to/from completed
            else if task.status != new_status {
                task.status = new_status;
            }
        }

        // Update timestamp
        task.updated_at = Utc::now();

        // Save and return
        self.task_repo.update(&task).await?;
        Ok(Self::task_to_response(&task))
    }

    /// Delete a task for a user
    async fn delete_task(&self, user_id: Uuid, task_id: Uuid) -> Result<(), DomainError> {
        // Get task first to ensure ownership
        let _task = self.task_repo.get_by_user_and_id(user_id, task_id).await?;

        // Delete task by ID
        self.task_repo.delete(task_id).await
    }
}
