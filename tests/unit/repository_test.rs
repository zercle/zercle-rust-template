//! Unit tests for repository error handling and edge cases
//!
//! These tests verify repository behavior under error conditions
//! and pagination boundaries using mock repositories.

use chrono::Utc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use anyhow::{bail, Result};
use async_trait::async_trait;
use zercle_rust_template::domain::entities::{
    CreateTask, CreateUser, Task, TaskPriority, TaskStatus, User,
};
use zercle_rust_template::domain::repositories::{TaskRepository, UserRepository};

// =========================================================================
// User Repository Error Handling Tests
// =========================================================================

mod user_repository_error_tests {
    use super::*;

    /// Mock user repository that simulates errors
    struct MockUserRepositoryWithErrors {
        users: Mutex<Vec<User>>,
        should_error: Arc<Mutex<bool>>,
    }

    impl MockUserRepositoryWithErrors {
        fn new(should_error: bool) -> Self {
            Self {
                users: Mutex::new(Vec::new()),
                should_error: Arc::new(Mutex::new(should_error)),
            }
        }

        fn set_error(&self, error: bool) {
            let mut guard = self.should_error.lock().unwrap();
            *guard = error;
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepositoryWithErrors {
        async fn create(&self, user: &CreateUser) -> Result<User> {
            if *self.should_error.lock().unwrap() {
                bail!("Database connection error");
            }
            let mut users = self.users.lock().unwrap();
            let id = Uuid::new_v4();
            let now = Utc::now();
            let new_user = User {
                id,
                email: user.email.clone(),
                password_hash: user.password_hash.clone(),
                full_name: user.full_name.clone(),
                phone: user.phone.clone(),
                created_at: now,
                updated_at: now,
            };
            users.push(new_user.clone());
            Ok(new_user)
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
            if *self.should_error.lock().unwrap() {
                bail!("Database query error");
            }
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.id == id).cloned())
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
            if *self.should_error.lock().unwrap() {
                bail!("Database query error");
            }
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.email == email).cloned())
        }

        async fn update(&self, user: &User) -> Result<User> {
            if *self.should_error.lock().unwrap() {
                bail!("Database update error");
            }
            let mut users = self.users.lock().unwrap();
            if let Some(idx) = users.iter().position(|u| u.id == user.id) {
                users[idx] = user.clone();
                Ok(users[idx].clone())
            } else {
                bail!("User not found");
            }
        }

        async fn delete(&self, id: Uuid) -> Result<()> {
            if *self.should_error.lock().unwrap() {
                bail!("Database delete error");
            }
            let mut users = self.users.lock().unwrap();
            users.retain(|u| u.id != id);
            Ok(())
        }

        async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<User>, i64)> {
            if *self.should_error.lock().unwrap() {
                bail!("Database list error");
            }
            let users = self.users.lock().unwrap();
            let total = users.len() as i64;
            let users: Vec<User> = users
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok((users, total))
        }

        async fn count(&self) -> Result<i64> {
            if *self.should_error.lock().unwrap() {
                bail!("Database count error");
            }
            let users = self.users.lock().unwrap();
            Ok(users.len() as i64)
        }
    }

    /// Test user repository create error handling
    #[tokio::test]
    async fn test_user_repository_create_error() {
        let mock_repo = Arc::new(MockUserRepositoryWithErrors::new(true));

        let create_user = CreateUser::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("Test User".to_string()),
            None,
        );

        let result = mock_repo.create(&create_user).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database connection error"));
    }

    /// Test user repository find_by_id error handling
    #[tokio::test]
    async fn test_user_repository_find_by_id_error() {
        let mock_repo = Arc::new(MockUserRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let user_id = Uuid::new_v4();
        let result = mock_repo.find_by_id(user_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database query error"));
    }

    /// Test user repository update error handling
    #[tokio::test]
    async fn test_user_repository_update_error() {
        let mock_repo = Arc::new(MockUserRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let user = User::new(
            Uuid::new_v4(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("Test User".to_string()),
            None,
        );

        let result = mock_repo.update(&user).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database update error"));
    }

    /// Test user repository delete error handling
    #[tokio::test]
    async fn test_user_repository_delete_error() {
        let mock_repo = Arc::new(MockUserRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let user_id = Uuid::new_v4();
        let result = mock_repo.delete(user_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database delete error"));
    }

    /// Test user repository list error handling
    #[tokio::test]
    async fn test_user_repository_list_error() {
        let mock_repo = Arc::new(MockUserRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let result = mock_repo.list(10, 0).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database list error"));
    }
}

// =========================================================================
// Task Repository Error Handling Tests
// =========================================================================

mod task_repository_error_tests {
    use super::*;

    /// Mock task repository that simulates errors
    struct MockTaskRepositoryWithErrors {
        tasks: Mutex<Vec<Task>>,
        should_error: Arc<Mutex<bool>>,
    }

    impl MockTaskRepositoryWithErrors {
        fn new(should_error: bool) -> Self {
            Self {
                tasks: Mutex::new(Vec::new()),
                should_error: Arc::new(Mutex::new(should_error)),
            }
        }

        fn set_error(&self, error: bool) {
            let mut guard = self.should_error.lock().unwrap();
            *guard = error;
        }
    }

    #[async_trait]
    impl TaskRepository for MockTaskRepositoryWithErrors {
        async fn create(&self, task: &CreateTask) -> Result<Task> {
            if *self.should_error.lock().unwrap() {
                bail!("Database connection error");
            }
            let mut tasks = self.tasks.lock().unwrap();
            let id = Uuid::new_v4();
            let now = Utc::now();
            let new_task = Task {
                id,
                user_id: task.user_id,
                title: task.title.clone(),
                description: task.description.clone(),
                status: TaskStatus::Pending,
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
            if *self.should_error.lock().unwrap() {
                bail!("Database query error");
            }
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().find(|t| t.id == id).cloned())
        }

        async fn find_by_user_id(
            &self,
            user_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> Result<(Vec<Task>, i64)> {
            if *self.should_error.lock().unwrap() {
                bail!("Database query error");
            }
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
            if *self.should_error.lock().unwrap() {
                bail!("Database update error");
            }
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(idx) = tasks.iter().position(|t| t.id == task.id) {
                tasks[idx] = task.clone();
                Ok(tasks[idx].clone())
            } else {
                bail!("Task not found");
            }
        }

        async fn delete(&self, id: Uuid, _user_id: Uuid) -> Result<()> {
            if *self.should_error.lock().unwrap() {
                bail!("Database delete error");
            }
            let mut tasks = self.tasks.lock().unwrap();
            tasks.retain(|t| t.id != id);
            Ok(())
        }

        async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64> {
            if *self.should_error.lock().unwrap() {
                bail!("Database count error");
            }
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().filter(|t| t.user_id == user_id).count() as i64)
        }
    }

    /// Test task repository create error handling
    #[tokio::test]
    async fn test_task_repository_create_error() {
        let mock_repo = Arc::new(MockTaskRepositoryWithErrors::new(true));

        let create_task = CreateTask::new(
            Uuid::new_v4(),
            "Test Task".to_string(),
            None,
            TaskPriority::Medium,
            None,
        );

        let result = mock_repo.create(&create_task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database connection error"));
    }

    /// Test task repository find_by_id error handling
    #[tokio::test]
    async fn test_task_repository_find_by_id_error() {
        let mock_repo = Arc::new(MockTaskRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let task_id = Uuid::new_v4();
        let result = mock_repo.find_by_id(task_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database query error"));
    }

    /// Test task repository update error handling
    #[tokio::test]
    async fn test_task_repository_update_error() {
        let mock_repo = Arc::new(MockTaskRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            None,
            TaskPriority::Medium,
            None,
        );

        let result = mock_repo.update(&task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database update error"));
    }

    /// Test task repository delete error handling
    #[tokio::test]
    async fn test_task_repository_delete_error() {
        let mock_repo = Arc::new(MockTaskRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let task_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let result = mock_repo.delete(task_id, user_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database delete error"));
    }

    /// Test task repository find_by_user_id error handling
    #[tokio::test]
    async fn test_task_repository_find_by_user_id_error() {
        let mock_repo = Arc::new(MockTaskRepositoryWithErrors::new(false));
        mock_repo.set_error(true);

        let user_id = Uuid::new_v4();
        let result = mock_repo.find_by_user_id(user_id, 10, 0).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Database query error"));
    }
}

// =========================================================================
// Repository Pagination Edge Cases Tests
// =========================================================================

mod pagination_edge_cases_tests {
    use super::*;

    /// Mock user repository for pagination testing
    struct MockUserRepositoryPagination {
        users: Mutex<Vec<User>>,
    }

    impl MockUserRepositoryPagination {
        fn new() -> Self {
            Self {
                users: Mutex::new(Vec::new()),
            }
        }

        fn add_users(&self, count: usize) {
            let mut users = self.users.lock().unwrap();
            for i in 0..count {
                let user = User::new(
                    Uuid::new_v4(),
                    format!("user{}@example.com", i),
                    "hashed_password".to_string(),
                    Some(format!("User {}", i)),
                    None,
                );
                users.push(user);
            }
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepositoryPagination {
        async fn create(&self, user: &CreateUser) -> Result<User> {
            let mut users = self.users.lock().unwrap();
            let id = Uuid::new_v4();
            let now = Utc::now();
            let new_user = User {
                id,
                email: user.email.clone(),
                password_hash: user.password_hash.clone(),
                full_name: user.full_name.clone(),
                phone: user.phone.clone(),
                created_at: now,
                updated_at: now,
            };
            users.push(new_user.clone());
            Ok(new_user)
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.id == id).cloned())
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
            let users = self.users.lock().unwrap();
            Ok(users.iter().find(|u| u.email == email).cloned())
        }

        async fn update(&self, user: &User) -> Result<User> {
            let mut users = self.users.lock().unwrap();
            if let Some(idx) = users.iter().position(|u| u.id == user.id) {
                users[idx] = user.clone();
                Ok(users[idx].clone())
            } else {
                bail!("User not found");
            }
        }

        async fn delete(&self, id: Uuid) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            users.retain(|u| u.id != id);
            Ok(())
        }

        async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<User>, i64)> {
            let users = self.users.lock().unwrap();
            let total = users.len() as i64;
            let users: Vec<User> = users
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok((users, total))
        }

        async fn count(&self) -> Result<i64> {
            let users = self.users.lock().unwrap();
            Ok(users.len() as i64)
        }
    }

    /// Test pagination with zero limit
    #[tokio::test]
    async fn test_pagination_zero_limit() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(10);

        let result = mock_repo.list(0, 0).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 0); // Zero limit returns empty
        assert_eq!(total, 10);
    }

    /// Test pagination with negative offset
    #[tokio::test]
    async fn test_pagination_negative_offset() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(10);

        // Negative offset should be handled by the use case, not repo
        // This test verifies the repo accepts any i64 value
        // Note: skip with negative value returns 0 items
        let result = mock_repo.list(10, -5).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 0); // skip(-5) returns 0 items
        assert_eq!(total, 10);
    }

    /// Test pagination with offset beyond total
    #[tokio::test]
    async fn test_pagination_offset_beyond_total() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(10);

        let result = mock_repo.list(10, 100).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 0); // Offset beyond data returns empty
        assert_eq!(total, 10);
    }

    /// Test pagination with limit larger than total
    #[tokio::test]
    async fn test_pagination_limit_larger_than_total() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(5);

        let result = mock_repo.list(100, 0).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 5); // Returns all available users
        assert_eq!(total, 5);
    }

    /// Test pagination with exact offset at end
    #[tokio::test]
    async fn test_pagination_offset_at_end() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(10);

        let result = mock_repo.list(10, 10).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 0); // Offset at end returns empty
        assert_eq!(total, 10);
    }

    /// Test pagination middle page
    #[tokio::test]
    async fn test_pagination_middle_page() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(100);

        let result = mock_repo.list(10, 50).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 10); // 10 users per page
        assert_eq!(total, 100);
    }

    /// Test pagination last page partial
    #[tokio::test]
    async fn test_pagination_last_page_partial() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(25);

        // Get page 2 (offset 10) with page size 10
        let result = mock_repo.list(10, 10).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 10);
        assert_eq!(total, 25);

        // Get page 3 (offset 20) with page size 10 - should be last page with 5 items
        let result = mock_repo.list(10, 20).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 5); // Only 5 items remaining
        assert_eq!(total, 25);
    }

    /// Test pagination with single item
    #[tokio::test]
    async fn test_pagination_single_item() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());
        mock_repo.add_users(1);

        let result = mock_repo.list(10, 0).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(total, 1);
    }

    /// Test pagination with empty dataset
    #[tokio::test]
    async fn test_pagination_empty_dataset() {
        let mock_repo = Arc::new(MockUserRepositoryPagination::new());

        let result = mock_repo.list(10, 0).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 0);
        assert_eq!(total, 0);
    }
}
