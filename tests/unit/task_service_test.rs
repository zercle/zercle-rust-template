//! Unit tests for TaskService
//!
//! Tests the business logic for task CRUD operations, status transitions,
// priority handling, and authorization checks.

use std::sync::Arc;
use uuid::Uuid;

use mockall::predicate::*;
use tokio_test;

use zercle_rust_template::internal::domain::{
    error::DomainError,
    task::{
        dto::{CreateTaskRequest, UpdateTaskRequest},
        entity::{Task, TaskPriority, TaskStatus},
        service::TaskServiceImpl,
        traits::TaskRepository,
        traits::TaskService,
    },
};

/// Mock task repository for testing
#[derive(Debug, Clone)]
struct MockTaskRepositoryImpl;

impl MockTaskRepositoryImpl {
    #[allow(dead_code)]
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl TaskRepository for MockTaskRepositoryImpl {
    async fn create(&self, _task: &Task) -> Result<(), DomainError> {
        Ok(())
    }

    async fn get_by_id(&self, _id: Uuid) -> Result<Task, DomainError> {
        Err(DomainError::TaskNotFound)
    }

    async fn get_by_user_and_id(&self, _user_id: Uuid, _task_id: Uuid) -> Result<Task, DomainError> {
        Err(DomainError::TaskNotFound)
    }

    async fn list_by_user(
        &self,
        _user_id: Uuid,
        _offset: u64,
        _limit: u64,
    ) -> Result<(Vec<Task>, u64), DomainError> {
        Ok((Vec::new(), 0))
    }

    async fn update(&self, _task: &Task) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete_by_user(&self, _user_id: Uuid) -> Result<u64, DomainError> {
        Ok(0)
    }

    async fn count_by_user(&self, _user_id: Uuid) -> Result<u64, DomainError> {
        Ok(0)
    }
}

/// Helper to create TaskService with mocked dependencies
#[allow(dead_code)]
fn create_task_service() -> (TaskServiceImpl, MockTaskRepository) {
    let task_repo = MockTaskRepository::new();
    let service = TaskServiceImpl::new(Arc::new(task_repo.clone()));
    (service, task_repo)
}

#[tokio::test]
async fn test_create_task_success() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();

    task_repo.expect_create().returning(|_, _| Ok(()));

    let create_req = CreateTaskRequest {
        title: "New Task".to_string(),
        description: Some("Task description".to_string()),
        status: None,
        priority: None,
        due_date: None,
    };

    // Act
    let result = service.create_task(user_id, create_req).await;

    // Assert
    assert!(result.is_ok(), "Create task should succeed");
    let response = result.unwrap();
    assert_eq!(response.title, "New Task");
    assert_eq!(response.description, Some("Task description".to_string()));
    assert_eq!(response.status, "pending"); // Default status
    assert_eq!(response.priority, "medium"); // Default priority
    assert_eq!(response.user_id, user_id);
}

#[tokio::test]
async fn test_create_task_with_custom_status_and_priority() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();

    task_repo.expect_create().returning(|_, _| Ok(()));

    let create_req = CreateTaskRequest {
        title: "Urgent Task".to_string(),
        description: Some("High priority task".to_string()),
        status: Some("in_progress".to_string()),
        priority: Some("urgent".to_string()),
        due_date: None,
    };

    // Act
    let result = service.create_task(user_id, create_req).await;

    // Assert
    assert!(result.is_ok(), "Create task should succeed with custom values");
    let response = result.unwrap();
    assert_eq!(response.status, "in_progress");
    assert_eq!(response.priority, "urgent");
}

#[tokio::test]
async fn test_get_task_success() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let existing_task = Task::new(
        task_id,
        user_id,
        "Test Task".to_string(),
        Some("Description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    );

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(move |_, _| Ok(existing_task.clone()));

    // Act
    let result = service.get_task(user_id, task_id).await;

    // Assert
    assert!(result.is_ok(), "Get task should succeed");
    let response = result.unwrap();
    assert_eq!(response.id, task_id);
    assert_eq!(response.title, "Test Task");
}

#[tokio::test]
async fn test_get_task_not_found() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(|_, _| Err(DomainError::TaskNotFound));

    // Act
    let result = service.get_task(user_id, task_id).await;

    // Assert
    assert!(result.is_err(), "Get task should fail when not found");
    assert!(matches!(result.unwrap_err(), DomainError::TaskNotFound));
}

#[tokio::test]
async fn test_get_task_unauthorized() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let different_user_id = Uuid::new_v4();

    let existing_task = Task::new(
        task_id,
        user_id, // Task belongs to original user
        "Test Task".to_string(),
        Some("Description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    );

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(different_user_id), eq(task_id))
        .returning(move |_, _| Err(DomainError::TaskNotFound));

    // Act
    let result = service.get_task(different_user_id, task_id).await;

    // Assert
    assert!(result.is_err(), "Get task should fail for unauthorized user");
    assert!(matches!(result.unwrap_err(), DomainError::TaskNotFound));
}

#[tokio::test]
async fn test_list_tasks_success() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let tasks = vec![
        Task::new(
            Uuid::new_v4(),
            user_id,
            "Task 1".to_string(),
            Some("Description 1".to_string()),
            TaskStatus::Pending,
            TaskPriority::Medium,
            None,
        ),
        Task::new(
            Uuid::new_v4(),
            user_id,
            "Task 2".to_string(),
            Some("Description 2".to_string()),
            TaskStatus::InProgress,
            TaskPriority::High,
            None,
        ),
    ];

    task_repo
        .expect_list_by_user()
        .with(eq(user_id), eq(0), eq(10))
        .returning(move |_, _, _| Ok((tasks.clone(), 2)));

    // Act
    let result = service.list_tasks(user_id, 1, 10).await;

    // Assert
    assert!(result.is_ok(), "List tasks should succeed");
    let response = result.unwrap();
    assert_eq!(response.tasks.len(), 2);
    assert_eq!(response.total, 2);
    assert_eq!(response.page, 1);
    assert_eq!(response.per_page, 10);
}

#[tokio::test]
async fn test_list_tasks_empty() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();

    task_repo
        .expect_list_by_user()
        .with(eq(user_id), eq(0), eq(10))
        .returning(|_, _, _| Ok((Vec::new(), 0)));

    // Act
    let result = service.list_tasks(user_id, 1, 10).await;

    // Assert
    assert!(result.is_ok(), "List tasks should succeed even when empty");
    let response = result.unwrap();
    assert_eq!(response.tasks.len(), 0);
    assert_eq!(response.total, 0);
}

#[tokio::test]
async fn test_list_tasks_pagination() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();

    // Second page
    task_repo
        .expect_list_by_user()
        .with(eq(user_id), eq(10), eq(10))
        .returning(|_, _, _| Ok((Vec::new(), 15)));

    // Act
    let result = service.list_tasks(user_id, 2, 10).await;

    // Assert
    assert!(result.is_ok(), "List tasks pagination should work");
    let response = result.unwrap();
    assert_eq!(response.page, 2);
    assert_eq!(response.per_page, 10);
    assert_eq!(response.total, 15);
}

#[tokio::test]
async fn test_update_task_status_to_completed() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    let mut existing_task = Task::new(
        task_id,
        user_id,
        "Task to Complete".to_string(),
        Some("Description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    );

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(move |_, _| Ok(existing_task.clone()));

    task_repo.expect_update().returning(|_, _| Ok(()));

    let update_req = UpdateTaskRequest {
        title: None,
        description: None,
        status: Some("completed".to_string()),
        priority: None,
        due_date: None,
    };

    // Act
    let result = service.update_task(user_id, task_id, update_req).await;

    // Assert
    assert!(result.is_ok(), "Update task to completed should succeed");
    let response = result.unwrap();
    assert_eq!(response.status, "completed");
    assert!(response.completed_at.is_some(), "completed_at should be set");
}

#[tokio::test]
async fn test_update_task_reopen() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let completed_time = chrono::Utc::now();

    let mut existing_task = Task::new(
        task_id,
        user_id,
        "Completed Task".to_string(),
        Some("Description".to_string()),
        TaskStatus::Completed,
        TaskPriority::Medium,
        None,
    );
    existing_task.completed_at = Some(completed_time);

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(move |_, _| Ok(existing_task.clone()));

    task_repo.expect_update().returning(|_, _| Ok(()));

    let update_req = UpdateTaskRequest {
        title: None,
        description: None,
        status: Some("pending".to_string()),
        priority: None,
        due_date: None,
    };

    // Act
    let result = service.update_task(user_id, task_id, update_req).await;

    // Assert
    assert!(result.is_ok(), "Reopen task should succeed");
    let response = result.unwrap();
    assert_eq!(response.status, "pending");
    assert!(response.completed_at.is_none(), "completed_at should be cleared");
}

#[tokio::test]
async fn test_update_task_title_and_priority() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    let existing_task = Task::new(
        task_id,
        user_id,
        "Original Title".to_string(),
        Some("Description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    );

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(move |_, _| Ok(existing_task.clone()));

    task_repo.expect_update().returning(|_, _| Ok(()));

    let update_req = UpdateTaskRequest {
        title: Some("Updated Title".to_string()),
        description: None,
        status: None,
        priority: Some("high".to_string()),
        due_date: None,
    };

    // Act
    let result = service.update_task(user_id, task_id, update_req).await;

    // Assert
    assert!(result.is_ok(), "Update task should succeed");
    let response = result.unwrap();
    assert_eq!(response.title, "Updated Title");
    assert_eq!(response.priority, "high");
}

#[tokio::test]
async fn test_delete_task_success() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    let existing_task = Task::new(
        task_id,
        user_id,
        "Task to Delete".to_string(),
        Some("Description".to_string()),
        TaskStatus::Pending,
        TaskPriority::Medium,
        None,
    );

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(move |_, _| Ok(existing_task.clone()));

    task_repo
        .expect_delete()
        .with(eq(task_id))
        .returning(|_| Ok(()));

    // Act
    let result = service.delete_task(user_id, task_id).await;

    // Assert
    assert!(result.is_ok(), "Delete task should succeed");
}

#[tokio::test]
async fn test_delete_task_not_found() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(user_id), eq(task_id))
        .returning(|_, _| Err(DomainError::TaskNotFound));

    // Act
    let result = service.delete_task(user_id, task_id).await;

    // Assert
    assert!(result.is_err(), "Delete task should fail when not found");
    assert!(matches!(result.unwrap_err(), DomainError::TaskNotFound));
}

#[tokio::test]
async fn test_delete_task_unauthorized() {
    // Arrange
    let (service, mut task_repo) = create_task_service();
    let user_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let different_user_id = Uuid::new_v4();

    task_repo
        .expect_get_by_user_and_id()
        .with(eq(different_user_id), eq(task_id))
        .returning(|_, _| Err(DomainError::TaskNotFound));

    // Act
    let result = service.delete_task(different_user_id, task_id).await;

    // Assert
    assert!(result.is_err(), "Delete task should fail for unauthorized user");
    assert!(matches!(result.unwrap_err(), DomainError::TaskNotFound));
}
