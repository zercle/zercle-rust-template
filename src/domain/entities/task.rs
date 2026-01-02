use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use validator::{Validate, ValidationErrors};

/// Errors that can occur during task entity validation
#[derive(Debug, Error)]
pub enum TaskValidationError {
    #[error("Title validation failed: {0}")]
    TitleError(String),
    
    #[error("Description validation failed: {0}")]
    DescriptionError(String),
    
    #[error("Due date validation failed: {0}")]
    DueDateError(String),
    
    #[error("Status validation failed: {0}")]
    StatusError(String),
    
    #[error("Priority validation failed: {0}")]
    PriorityError(String),
}

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is pending and not yet started
    Pending,
    /// Task is currently in progress
    InProgress,
    /// Task has been completed
    Completed,
    /// Task has been cancelled
    Cancelled,
}

impl TaskStatus {
    /// Get all possible task statuses
    ///
    /// # Returns
    /// A vector of all TaskStatus variants
    pub fn all() -> Vec<TaskStatus> {
        vec![
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::Cancelled,
        ]
    }
    
    /// Check if the status is a terminal state
    ///
    /// # Returns
    /// `true` if the status is Completed or Cancelled
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Cancelled)
    }
    
    /// Check if the status is active
    ///
    /// # Returns
    /// `true` if the status is Pending or InProgress
    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::Pending | TaskStatus::InProgress)
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

/// Task priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    /// Low priority task
    Low,
    /// Medium priority task
    Medium,
    /// High priority task
    High,
    /// Urgent priority task
    Urgent,
}

impl TaskPriority {
    /// Get all possible task priorities
    ///
    /// # Returns
    /// A vector of all TaskPriority variants
    pub fn all() -> Vec<TaskPriority> {
        vec![
            TaskPriority::Low,
            TaskPriority::Medium,
            TaskPriority::High,
            TaskPriority::Urgent,
        ]
    }
    
    /// Get the numeric value of the priority for sorting
    ///
    /// # Returns
    /// A u8 value representing the priority level (higher = more urgent)
    pub fn value(&self) -> u8 {
        match self {
            TaskPriority::Low => 1,
            TaskPriority::Medium => 2,
            TaskPriority::High => 3,
            TaskPriority::Urgent => 4,
        }
    }
    
    /// Create TaskPriority from a numeric value
    ///
    /// # Arguments
    /// * `value` - Numeric value (1-4)
    ///
    /// # Returns
    /// `Option<TaskPriority>` - The corresponding priority or None if invalid
    pub fn from_value(value: u8) -> Option<TaskPriority> {
        match value {
            1 => Some(TaskPriority::Low),
            2 => Some(TaskPriority::Medium),
            3 => Some(TaskPriority::High),
            4 => Some(TaskPriority::Urgent),
            _ => None,
        }
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Medium
    }
}

/// Task entity representing a task in the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task entity
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the task
    /// * `user_id` - ID of the user who owns the task
    /// * `title` - Task title
    /// * `description` - Optional task description
    /// * `priority` - Task priority
    /// * `due_date` - Optional due date
    ///
    /// # Returns
    /// A new Task instance with current timestamps and default status
    pub fn new(
        id: Uuid,
        user_id: Uuid,
        title: String,
        description: Option<String>,
        priority: TaskPriority,
        due_date: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            title,
            description,
            status: TaskStatus::Pending,
            priority,
            due_date,
            completed_at: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Mark the task as completed
    ///
    /// # Returns
    /// `true` if the status was changed, `false` if already completed
    pub fn mark_completed(&mut self) -> bool {
        if self.status == TaskStatus::Completed {
            return false;
        }
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        true
    }
    
    /// Update the task status
    ///
    /// # Arguments
    /// * `status` - New status
    ///
    /// # Returns
    /// `true` if the status was changed, `false` if already set
    pub fn update_status(&mut self, status: TaskStatus) -> bool {
        if self.status == status {
            return false;
        }
        self.status = status;
        self.updated_at = Utc::now();
        
        // Update completed_at if status is completed
        if status == TaskStatus::Completed {
            self.completed_at = Some(Utc::now());
        } else {
            self.completed_at = None;
        }
        
        true
    }
    
    /// Check if the task is overdue
    ///
    /// # Returns
    /// `true` if the task has a due date and it's in the past, and the task is not completed
    pub fn is_overdue(&self) -> bool {
        if let Some(due_date) = self.due_date {
            return due_date < Utc::now() && self.status != TaskStatus::Completed;
        }
        false
    }
}

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTaskRequest {
    /// Task title (min 1, max 255 characters)
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,
    
    /// Optional task description
    #[validate(length(max = 5000, message = "Description must not exceed 5000 characters"))]
    pub description: Option<String>,
    
    /// Task priority (defaults to Medium if not specified)
    pub priority: Option<TaskPriority>,
    
    /// Optional due date
    pub due_date: Option<DateTime<Utc>>,
}

impl CreateTaskRequest {
    /// Validate the create task request
    ///
    /// # Returns
    /// `Result<(), TaskValidationError>` - Ok if valid, error otherwise
    pub fn validate_request(&self) -> Result<(), TaskValidationError> {
        // Run validator crate's built-in validations
        if let Err(errors) = self.validate() {
            let field_errors: ValidationErrors = errors;
            for (field, error_messages) in field_errors.field_errors() {
                for msg in error_messages {
                    if field == "title" {
                        return Err(TaskValidationError::TitleError(
                            msg.message.as_deref().unwrap_or("Invalid title").to_string()
                        ));
                    } else if field == "description" {
                        return Err(TaskValidationError::DescriptionError(
                            msg.message.as_deref().unwrap_or("Invalid description").to_string()
                        ));
                    }
                }
            }
        }
        
        // Validate due date is in the future
        if let Some(due_date) = self.due_date {
            if due_date < Utc::now() {
                return Err(TaskValidationError::DueDateError(
                    "Due date must be in the future".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Get the priority with default fallback
    ///
    /// # Returns
    /// The specified priority or Medium if not set
    pub fn get_priority(&self) -> TaskPriority {
        self.priority.unwrap_or_default()
    }
}

/// Request to update an existing task
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateTaskRequest {
    /// Optional new title
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: Option<String>,
    
    /// Optional new description
    #[validate(length(max = 5000, message = "Description must not exceed 5000 characters"))]
    pub description: Option<String>,
    
    /// Optional new status
    pub status: Option<TaskStatus>,
    
    /// Optional new priority
    pub priority: Option<TaskPriority>,
    
    /// Optional new due date
    pub due_date: Option<DateTime<Utc>>,
}

impl UpdateTaskRequest {
    /// Validate the update task request
    ///
    /// # Returns
    /// `Result<(), TaskValidationError>` - Ok if valid, error otherwise
    pub fn validate_request(&self) -> Result<(), TaskValidationError> {
        // Run validator crate's built-in validations
        if let Err(errors) = self.validate() {
            let field_errors: ValidationErrors = errors;
            for (field, error_messages) in field_errors.field_errors() {
                for msg in error_messages {
                    if field == "title" {
                        return Err(TaskValidationError::TitleError(
                            msg.message.as_deref().unwrap_or("Invalid title").to_string()
                        ));
                    } else if field == "description" {
                        return Err(TaskValidationError::DescriptionError(
                            msg.message.as_deref().unwrap_or("Invalid description").to_string()
                        ));
                    }
                }
            }
        }
        
        // Validate due date is in the future if provided
        if let Some(due_date) = self.due_date {
            if due_date < Utc::now() {
                return Err(TaskValidationError::DueDateError(
                    "Due date must be in the future".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if the update request has any fields to update
    ///
    /// # Returns
    /// `true` if at least one field is set, `false` otherwise
    pub fn has_updates(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.status.is_some()
            || self.priority.is_some()
            || self.due_date.is_some()
    }
}

/// Data structure for creating a new task (repository layer)
#[derive(Debug, Clone)]
pub struct CreateTask {
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
}

impl CreateTask {
    /// Create a new CreateTask instance
    pub fn new(
        user_id: Uuid,
        title: String,
        description: Option<String>,
        priority: TaskPriority,
        due_date: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            user_id,
            title,
            description,
            priority,
            due_date,
        }
    }
}

/// Data structure for updating a task (repository layer)
#[derive(Debug, Clone)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<Option<DateTime<Utc>>>,
}

impl UpdateTask {
    /// Create a new UpdateTask instance
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            status: None,
            priority: None,
            due_date: None,
            completed_at: None,
        }
    }
    
    /// Set the title
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    /// Set the description
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }
    
    /// Set the status
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }
    
    /// Set the priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = Some(priority);
        self
    }
    
    /// Set the due date
    pub fn with_due_date(mut self, due_date: Option<DateTime<Utc>>) -> Self {
        self.due_date = due_date;
        self
    }
    
    /// Set the completed_at timestamp
    pub fn with_completed_at(mut self, completed_at: Option<DateTime<Utc>>) -> Self {
        self.completed_at = Some(completed_at);
        self
    }
    
    /// Check if there are any updates
    pub fn has_updates(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.status.is_some()
            || self.priority.is_some()
            || self.due_date.is_some()
            || self.completed_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_all() {
        let statuses = TaskStatus::all();
        assert_eq!(statuses.len(), 4);
        assert!(statuses.contains(&TaskStatus::Pending));
        assert!(statuses.contains(&TaskStatus::InProgress));
        assert!(statuses.contains(&TaskStatus::Completed));
        assert!(statuses.contains(&TaskStatus::Cancelled));
    }

    #[test]
    fn test_task_status_is_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_task_status_is_active() {
        assert!(TaskStatus::Pending.is_active());
        assert!(TaskStatus::InProgress.is_active());
        assert!(!TaskStatus::Completed.is_active());
        assert!(!TaskStatus::Cancelled.is_active());
    }

    #[test]
    fn test_task_priority_value() {
        assert_eq!(TaskPriority::Low.value(), 1);
        assert_eq!(TaskPriority::Medium.value(), 2);
        assert_eq!(TaskPriority::High.value(), 3);
        assert_eq!(TaskPriority::Urgent.value(), 4);
    }

    #[test]
    fn test_task_priority_from_value() {
        assert_eq!(TaskPriority::from_value(1), Some(TaskPriority::Low));
        assert_eq!(TaskPriority::from_value(2), Some(TaskPriority::Medium));
        assert_eq!(TaskPriority::from_value(3), Some(TaskPriority::High));
        assert_eq!(TaskPriority::from_value(4), Some(TaskPriority::Urgent));
        assert_eq!(TaskPriority::from_value(5), None);
    }

    #[test]
    fn test_task_new() {
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let task = Task::new(
            id,
            user_id,
            "Test Task".to_string(),
            Some("Test Description".to_string()),
            TaskPriority::High,
            None,
        );

        assert_eq!(task.id, id);
        assert_eq!(task.user_id, user_id);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Test Description".to_string()));
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, TaskPriority::High);
        assert!(task.due_date.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_mark_completed() {
        let mut task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            None,
            TaskPriority::Medium,
            None,
        );

        assert!(task.mark_completed());
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(!task.mark_completed()); // Already completed
    }

    #[test]
    fn test_task_update_status() {
        let mut task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            None,
            TaskPriority::Medium,
            None,
        );

        assert!(task.update_status(TaskStatus::InProgress));
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(!task.update_status(TaskStatus::InProgress)); // Same status
    }

    #[test]
    fn test_task_is_overdue() {
        let mut task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            None,
            TaskPriority::Medium,
            Some(Utc::now() - chrono::Duration::hours(1)),
        );

        assert!(task.is_overdue());
        
        task.status = TaskStatus::Completed;
        assert!(!task.is_overdue());
    }

    #[test]
    fn test_create_task_request_valid() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Test Description".to_string()),
            priority: Some(TaskPriority::High),
            due_date: Some(Utc::now() + chrono::Duration::days(1)),
        };

        assert!(request.validate_request().is_ok());
    }

    #[test]
    fn test_create_task_request_invalid_title_empty() {
        let request = CreateTaskRequest {
            title: "".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_create_task_request_invalid_title_too_long() {
        let request = CreateTaskRequest {
            title: "a".repeat(256),
            description: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_create_task_request_invalid_due_date() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: Some(Utc::now() - chrono::Duration::days(1)),
        };

        assert!(request.validate_request().is_err());
    }

    #[test]
    fn test_create_task_request_get_priority() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: Some(TaskPriority::High),
            due_date: None,
        };

        assert_eq!(request.get_priority(), TaskPriority::High);
    }

    #[test]
    fn test_create_task_request_get_priority_default() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            priority: None,
            due_date: None,
        };

        assert_eq!(request.get_priority(), TaskPriority::Medium);
    }

    #[test]
    fn test_update_task_request_valid() {
        let request = UpdateTaskRequest {
            title: Some("Updated Task".to_string()),
            description: Some("Updated Description".to_string()),
            status: Some(TaskStatus::Completed),
            priority: Some(TaskPriority::Urgent),
            due_date: Some(Utc::now() + chrono::Duration::days(2)),
        };

        assert!(request.validate_request().is_ok());
        assert!(request.has_updates());
    }

    #[test]
    fn test_update_task_request_empty() {
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            due_date: None,
        };

        assert!(request.validate_request().is_ok());
        assert!(!request.has_updates());
    }
}
