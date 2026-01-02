//! PostgreSQL repository implementations
//!
//! This module contains concrete implementations of User and Task repositories
//! using sqlx with PostgreSQL.

use crate::config::Settings;
use crate::domain::entities::{CreateTask, CreateUser, Task, TaskPriority, TaskStatus, User};
use crate::domain::repositories::{TaskRepository, UserRepository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgRow;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

/// Custom error types for repository operations
#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("User not found with id: {0}")]
    UserNotFound(Uuid),

    #[error("User not found with email: {0}")]
    UserNotFoundByEmail(String),

    #[error("User already exists with email: {0}")]
    UserAlreadyExists(String),

    #[error("Task not found with id: {0}")]
    TaskNotFound(Uuid),

    #[error("Task not found with id: {0} for user: {1}")]
    TaskNotFoundForUser(Uuid, Uuid),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

/// PostgreSQL user repository implementation
pub struct PostgresUserRepository {
    pool: Pool<Postgres>,
}

impl PostgresUserRepository {
    /// Create a new PostgresUserRepository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// A new PostgresUserRepository instance
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Create a new PostgresUserRepository from settings
    ///
    /// # Arguments
    /// * `settings` - Application settings
    ///
    /// # Returns
    /// Result containing the repository or an error
    pub async fn from_settings(settings: &Settings) -> Result<Self> {
        let pool = crate::infrastructure::db::connect(settings)
            .await
            .context("Failed to connect to database for PostgresUserRepository")?;
        Ok(Self::new(pool))
    }
}

/// Map a database row to a User entity
fn map_row_to_user(row: &PgRow) -> Result<User, sqlx::Error> {
    Ok(User {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        password_hash: row.try_get("password_hash")?,
        full_name: row.try_get("full_name")?,
        phone: row.try_get("phone")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    /// Create a new user in the database
    ///
    /// # Arguments
    /// * `user` - User creation data
    ///
    /// # Returns
    /// Result containing the created user or an error
    async fn create(&self, user: &CreateUser) -> Result<User> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let user_row = sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, full_name, phone, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.phone)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create user")?;

        map_row_to_user(&user_row).context("Failed to map user row")
    }

    /// Find a user by their ID
    ///
    /// # Arguments
    /// * `id` - User's UUID
    ///
    /// # Returns
    /// Result containing the user if found or None
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user_row = sqlx::query(
            r#"
            SELECT * FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find user by id")?;

        match user_row {
            Some(row) => map_row_to_user(&row)
                .map(Some)
                .context("Failed to map user row"),
            None => Ok(None),
        }
    }

    /// Find a user by their email
    ///
    /// # Arguments
    /// * `email` - User's email address
    ///
    /// # Returns
    /// Result containing the user if found or None
    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user_row = sqlx::query(
            r#"
            SELECT * FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find user by email")?;

        match user_row {
            Some(row) => map_row_to_user(&row)
                .map(Some)
                .context("Failed to map user row"),
            None => Ok(None),
        }
    }

    /// Update an existing user
    ///
    /// # Arguments
    /// * `user` - User entity with updated data
    ///
    /// # Returns
    /// Result containing the updated user or an error
    async fn update(&self, user: &User) -> Result<User> {
        let user_row = sqlx::query(
            r#"
            UPDATE users
            SET email = COALESCE($2, email),
                password_hash = COALESCE($3, password_hash),
                full_name = COALESCE($4, full_name),
                phone = COALESCE($5, phone),
                updated_at = $6
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.phone)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .context("Failed to update user")?;

        map_row_to_user(&user_row).context("Failed to map user row")
    }

    /// Delete a user by their ID
    ///
    /// # Arguments
    /// * `id` - User's UUID
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to delete user")?;

        Ok(())
    }

    /// List users with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of users to return
    /// * `offset` - Number of users to skip
    ///
    /// # Returns
    /// Result containing a tuple of users and total count
    async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<User>, i64)> {
        let users: Vec<User> = sqlx::query_as(
            r#"
            SELECT * FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list users")?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count users")?;

        Ok((users, total))
    }

    /// Count total users in the database
    ///
    /// # Returns
    /// Result containing the total count
    async fn count(&self) -> Result<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count users")?;

        Ok(total)
    }
}

/// PostgreSQL task repository implementation
pub struct PostgresTaskRepository {
    pool: Pool<Postgres>,
}

impl PostgresTaskRepository {
    /// Create a new PostgresTaskRepository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// A new PostgresTaskRepository instance
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Create a new PostgresTaskRepository from settings
    ///
    /// # Arguments
    /// * `settings` - Application settings
    ///
    /// # Returns
    /// Result containing the repository or an error
    pub async fn from_settings(settings: &Settings) -> Result<Self> {
        let pool = crate::infrastructure::db::connect(settings)
            .await
            .context("Failed to connect to database for PostgresTaskRepository")?;
        Ok(Self::new(pool))
    }
}

/// Parse task status from database string
fn parse_task_status(status: &str) -> TaskStatus {
    match status {
        "pending" => TaskStatus::Pending,
        "in_progress" => TaskStatus::InProgress,
        "completed" => TaskStatus::Completed,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}

/// Parse task priority from database string
fn parse_task_priority(priority: &str) -> TaskPriority {
    match priority {
        "low" => TaskPriority::Low,
        "medium" => TaskPriority::Medium,
        "high" => TaskPriority::High,
        "urgent" => TaskPriority::Urgent,
        _ => TaskPriority::Medium,
    }
}

/// Convert task status to database string
fn task_status_to_string(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Convert task priority to database string
fn task_priority_to_string(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "low",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
        TaskPriority::Urgent => "urgent",
    }
}

/// Map a database row to a Task entity
fn map_row_to_task(row: &PgRow) -> Result<Task, sqlx::Error> {
    let status_str: String = row.try_get("status")?;
    let priority_str: String = row.try_get("priority")?;

    let status = parse_task_status(&status_str);
    let priority = parse_task_priority(&priority_str);

    Ok(Task {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        status,
        priority,
        due_date: row.try_get("due_date")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[async_trait]
impl TaskRepository for PostgresTaskRepository {
    /// Create a new task in the database
    ///
    /// # Arguments
    /// * `task` - Task creation data
    ///
    /// # Returns
    /// Result containing the created task or an error
    async fn create(&self, task: &CreateTask) -> Result<Task> {
        let _id = Uuid::new_v4();
        let now = Utc::now();

        let status_str = "pending";
        let priority_str = task_priority_to_string(task.priority);

        let task_row = sqlx::query(
            r#"
            INSERT INTO tasks (user_id, title, description, status, priority, due_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(task.user_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(status_str)
        .bind(priority_str)
        .bind(task.due_date)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create task")?;

        map_row_to_task(&task_row).context("Failed to map task row")
    }

    /// Find a task by its ID
    ///
    /// # Arguments
    /// * `id` - Task's UUID
    ///
    /// # Returns
    /// Result containing the task if found or None
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Task>> {
        let task_row = sqlx::query(
            r#"
            SELECT * FROM tasks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find task by id")?;

        match task_row {
            Some(row) => map_row_to_task(&row)
                .map(Some)
                .context("Failed to map task row"),
            None => Ok(None),
        }
    }

    /// Find tasks by user ID with pagination
    ///
    /// # Arguments
    /// * `user_id` - User's UUID
    /// * `limit` - Maximum number of tasks to return
    /// * `offset` - Number of tasks to skip
    ///
    /// # Returns
    /// Result containing a tuple of tasks and total count
    async fn find_by_user_id(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Task>, i64)> {
        // First get the tasks
        let rows = sqlx::query(
            r#"
            SELECT * FROM tasks
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to find tasks by user id")?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(map_row_to_task(&row).context("Failed to map task row")?);
        }

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM tasks
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count tasks by user id")?;

        Ok((tasks, total))
    }

    /// Update an existing task
    ///
    /// # Arguments
    /// * `task` - Task entity with updated data
    ///
    /// # Returns
    /// Result containing the updated task or an error
    async fn update(&self, task: &Task) -> Result<Task> {
        let status_str = task_status_to_string(task.status);
        let priority_str = task_priority_to_string(task.priority);

        let task_row = sqlx::query(
            r#"
            UPDATE tasks
            SET title = COALESCE($2, title),
                description = COALESCE($3, description),
                status = COALESCE($4, status),
                priority = COALESCE($5, priority),
                due_date = COALESCE($6, due_date),
                completed_at = COALESCE($7, completed_at),
                updated_at = $8
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(task.id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(status_str)
        .bind(priority_str)
        .bind(task.due_date)
        .bind(task.completed_at)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .context("Failed to update task")?;

        map_row_to_task(&task_row).context("Failed to map task row")
    }

    /// Delete a task by its ID and user ID
    ///
    /// # Arguments
    /// * `id` - Task's UUID
    /// * `user_id` - User's UUID (ownership check)
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM tasks
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("Failed to delete task")?;

        Ok(())
    }

    /// Count tasks by user ID
    ///
    /// # Arguments
    /// * `user_id` - User's UUID
    ///
    /// # Returns
    /// Result containing the total count
    async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM tasks
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count tasks by user id")?;

        Ok(total)
    }
}
