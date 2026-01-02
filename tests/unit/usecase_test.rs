//! Unit tests for use case business logic
//!
//! These tests verify the business logic in use cases using mock repositories.

use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

use zercle_rust_template::domain::entities::{
    CreateUser, CreateTask, Task, TaskPriority, TaskStatus, User,
};
use zercle_rust_template::domain::repositories::{TaskRepository, UserRepository};
use zercle_rust_template::domain::usecases::{TaskUsecase, TaskUsecaseImpl, UserUsecase, UserUsecaseImpl};
use zercle_rust_template::config::Settings;
use async_trait::async_trait;
use anyhow::{Result, Context};

// Mock user repository for testing
struct MockUserRepository {
    users: Mutex<Vec<User>>,
}

impl MockUserRepository {
    fn new() -> Self {
        Self {
            users: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
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
            anyhow::bail!("User not found")
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

// Mock task repository for testing
struct MockTaskRepository {
    tasks: Mutex<Vec<Task>>,
}

impl MockTaskRepository {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }
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
            anyhow::bail!("Task not found")
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

mod user_usecase_tests {
    use super::*;
    use zercle_rust_template::domain::entities::{CreateUserRequest, LoginRequest};

    /// Test successful user registration
    #[tokio::test]
    async fn test_register_success() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo, &settings);

        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: Some("+1234567890".to_string()),
        };

        let result = usecase.register(req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.user.email, "test@example.com");
        assert!(!response.token.is_empty());
    }

    /// Test user registration with duplicate email
    #[tokio::test]
    async fn test_register_duplicate_email() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: None,
            phone: None,
        };

        // First registration should succeed
        assert!(usecase.register(req.clone()).await.is_ok());

        // Second registration with same email should fail
        let result = usecase.register(req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    /// Test successful login
    #[tokio::test]
    async fn test_login_success() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        // Register first
        let register_req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: None,
        };
        usecase.register(register_req).await.unwrap();

        // Then login
        let login_req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
        };

        let result = usecase.login(login_req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.user.email, "test@example.com");
        assert!(!response.token.is_empty());
    }

    /// Test login with invalid password
    #[tokio::test]
    async fn test_login_invalid_password() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        // Register first
        let register_req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: None,
            phone: None,
        };
        usecase.register(register_req).await.unwrap();

        // Then login with wrong password
        let login_req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "WrongPassword123!".to_string(),
        };

        let result = usecase.login(login_req).await;
        assert!(result.is_err());
    }

    /// Test login with non-existent user
    #[tokio::test]
    async fn test_login_user_not_found() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo, &settings);

        let login_req = LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "Password123!".to_string(),
        };

        let result = usecase.login(login_req).await;
        assert!(result.is_err());
    }

    /// Test get profile
    #[tokio::test]
    async fn test_get_profile() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo.clone(), &settings);

        // Register first
        let register_req = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: Some("Test User".to_string()),
            phone: None,
        };
        let auth_response = usecase.register(register_req).await.unwrap();

        // Get profile
        let result = usecase.get_profile(auth_response.user.id).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
    }

    /// Test get profile not found
    #[tokio::test]
    async fn test_get_profile_not_found() {
        let settings = Settings::from_env().unwrap();
        let mock_repo = Arc::new(MockUserRepository::new());
        let usecase = UserUsecaseImpl::new(mock_repo, &settings);

        let fake_id = Uuid::new_v4();
        let result = usecase.get_profile(fake_id).await;
        assert!(result.is_err());
    }
}

mod task_usecase_tests {
    use super::*;
    use zercle_rust_template::domain::entities::CreateTaskRequest;

    /// Test successful task creation
    #[tokio::test]
    async fn test_create_task_success() {
        let mock_repo = Arc::new(MockTaskRepository::new());
        let usecase = TaskUsecaseImpl::new(mock_repo);

        let user_id = Uuid::new_v4();
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Test Description".to_string()),
            priority: Some(TaskPriority::High),
            due_date: Some(Utc::now() + chrono::Duration::days(1)),
        };

        let result = usecase.create_task(user_id, req).await;
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.user_id, user_id);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    /// Test get task by ID
    #[tokio::test]
    async fn test_get_task_success() {
        let mock_repo = Arc::new(MockTaskRepository::new());
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

    /// Test get task not owned by user
    #[tokio::test]
    async fn test_get_task_not_owned() {
        let mock_repo = Arc::new(MockTaskRepository::new());
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
    }

    /// Test list tasks
    #[tokio::test]
    async fn test_list_tasks() {
        let mock_repo = Arc::new(MockTaskRepository::new());
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();

        // Create multiple tasks
        for i in 0..5 {
            let req = CreateTaskRequest {
                title: format!("Test Task {}", i),
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

    /// Test delete task
    #[tokio::test]
    async fn test_delete_task() {
        let mock_repo = Arc::new(MockTaskRepository::new());
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
    }

    /// Test delete task not owned
    #[tokio::test]
    async fn test_delete_task_not_owned() {
        let mock_repo = Arc::new(MockTaskRepository::new());
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

        let result = usecase.delete_task(created.id, other_user_id).await;
        assert!(result.is_err());
    }

    /// Test pagination limits
    #[tokio::test]
    async fn test_task_pagination_limits() {
        let mock_repo = Arc::new(MockTaskRepository::new());
        let usecase = TaskUsecaseImpl::new(mock_repo.clone());

        let user_id = Uuid::new_v4();

        // Create 10 tasks
        for i in 0..10 {
            let req = CreateTaskRequest {
                title: format!("Test Task {}", i),
                description: None,
                priority: None,
                due_date: None,
            };
            usecase.create_task(user_id, req).await.unwrap();
        }

        // Get first 5
        let result = usecase.list_tasks(user_id, 5, 0).await;
        assert!(result.is_ok());
        let (tasks, total) = result.unwrap();
        assert_eq!(tasks.len(), 5);
        assert_eq!(total, 10);

        // Get next 5
        let result = usecase.list_tasks(user_id, 5, 5).await;
        assert!(result.is_ok());
        let (tasks, _) = result.unwrap();
        assert_eq!(tasks.len(), 5);
    }
}
