use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::task::entity::{Task, TaskPriority, TaskStatus};
use crate::internal::domain::task::traits::{TaskRepository as TaskRepositoryTrait};

/// Task repository implementation using SQLx PostgreSQL
pub struct TaskRepository {
    pool: PgPool,
}

impl TaskRepository {
    /// Create a new TaskRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Helper function to convert SQLx UUID to domain Uuid
fn to_uuid(pg_uuid: sqlx::types::Uuid) -> Uuid {
    Uuid::from_bytes(pg_uuid.as_bytes().to_owned())
}

/// Helper function to convert domain Uuid to SQLx UUID
fn from_uuid(uuid: Uuid) -> sqlx::types::Uuid {
    sqlx::types::Uuid::from_bytes(*uuid.as_bytes())
}

/// Helper function to convert SQLx DateTime to domain DateTime<Utc>
fn to_datetime(ts: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>>) -> Option<DateTime<Utc>> {
    ts.map(|t| t.with_timezone(&Utc))
}

/// Helper function to convert domain DateTime<Utc> to SQLx DateTime
fn from_datetime(dt: Option<DateTime<Utc>>) -> Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>> {
    dt.map(|d| d.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
}

/// Convert database string to TaskStatus enum
fn to_task_status(status: String) -> TaskStatus {
    match status.as_str() {
        "pending" => TaskStatus::Pending,
        "in_progress" => TaskStatus::InProgress,
        "completed" => TaskStatus::Completed,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}

/// Convert TaskStatus enum to database string
fn from_task_status(status: &TaskStatus) -> String {
    status.to_string()
}

/// Convert database string to TaskPriority enum
fn to_task_priority(priority: String) -> TaskPriority {
    match priority.as_str() {
        "low" => TaskPriority::Low,
        "medium" => TaskPriority::Medium,
        "high" => TaskPriority::High,
        "urgent" => TaskPriority::Urgent,
        _ => TaskPriority::Medium,
    }
}

/// Convert TaskPriority enum to database string
fn from_task_priority(priority: &TaskPriority) -> String {
    priority.to_string()
}

#[async_trait]
impl TaskRepositoryTrait for TaskRepository {
    /// Create a new task in the database
    async fn create(&self, task: &Task) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO tasks (id, user_id, title, description, status, priority, due_date, completed_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            from_uuid(task.id),
            from_uuid(task.user_id),
            task.title,
            task.description,
            from_task_status(&task.status),
            from_task_priority(&task.priority),
            from_datetime(task.due_date),
            from_datetime(task.completed_at),
            from_datetime(task.created_at),
            from_datetime(task.updated_at)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get task by ID from the database
    async fn get_by_id(&self, id: Uuid) -> Result<Task, DomainError> {
        let row = sqlx::query_as!(
            DbTask,
            r#"
            SELECT id, user_id, title, description, status, priority, due_date, completed_at, created_at, updated_at
            FROM tasks
            WHERE id = $1
            "#,
            from_uuid(id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                DomainError::TaskNotFound
            } else {
                DomainError::Database(e.to_string())
            }
        })?;

        Ok(row.into_domain())
    }

    /// Get task by ID and user ID (for authorization)
    async fn get_by_user_and_id(&self, user_id: Uuid, task_id: Uuid) -> Result<Task, DomainError> {
        let row = sqlx::query_as!(
            DbTask,
            r#"
            SELECT id, user_id, title, description, status, priority, due_date, completed_at, created_at, updated_at
            FROM tasks
            WHERE id = $1 AND user_id = $2
            "#,
            from_uuid(task_id),
            from_uuid(user_id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                DomainError::TaskNotFound
            } else {
                DomainError::Database(e.to_string())
            }
        })?;

        Ok(row.into_domain())
    }

    /// List tasks for a user with pagination
    async fn list_by_user(
        &self,
        user_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> Result<(Vec<Task>, u64), DomainError> {
        let count: i64 = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM tasks
            WHERE user_id = $1
            "#,
            from_uuid(user_id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?
        .count
        .unwrap_or(0);

        let tasks = sqlx::query_as!(
            DbTask,
            r#"
            SELECT id, user_id, title, description, status, priority, due_date, completed_at, created_at, updated_at
            FROM tasks
            WHERE user_id = $1
            ORDER BY created_at DESC
            OFFSET $2 LIMIT $3
            "#,
            from_uuid(user_id),
            offset as i64,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        let tasks: Vec<Task> = tasks.into_iter().map(|t| t.into_domain()).collect();

        Ok((tasks, count as u64))
    }

    /// Update task in the database
    async fn update(&self, task: &Task) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE tasks
            SET title = $1, description = $2, status = $3, priority = $4, due_date = $5, completed_at = $6, updated_at = $7
            WHERE id = $8
            "#,
            task.title,
            task.description,
            from_task_status(&task.status),
            from_task_priority(&task.priority),
            from_datetime(task.due_date),
            from_datetime(task.completed_at),
            from_datetime(task.updated_at),
            from_uuid(task.id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete task by ID from the database
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM tasks
            WHERE id = $1
            "#,
            from_uuid(id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::TaskNotFound);
        }

        Ok(())
    }

    /// Delete all tasks for a user (cascade)
    async fn delete_by_user(&self, user_id: Uuid) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM tasks
            WHERE user_id = $1
            "#,
            from_uuid(user_id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Count tasks for a user
    async fn count_by_user(&self, user_id: Uuid) -> Result<u64, DomainError> {
        let count: i64 = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM tasks
            WHERE user_id = $1
            "#,
            from_uuid(user_id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?
        .count
        .unwrap_or(0);

        Ok(count as u64)
    }
}

/// Internal database task struct for SQLx mapping
struct DbTask {
    id: sqlx::types::Uuid,
    user_id: sqlx::types::Uuid,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    due_date: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>>,
    completed_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>>,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
}

impl DbTask {
    /// Convert database row to domain Task entity
    fn into_domain(self) -> Task {
        Task {
            id: to_uuid(self.id),
            user_id: to_uuid(self.user_id),
            title: self.title,
            description: self.description,
            status: to_task_status(self.status),
            priority: to_task_priority(self.priority),
            due_date: to_datetime(self.due_date),
            completed_at: to_datetime(self.completed_at),
            created_at: to_datetime(self.created_at).unwrap_or_else(Utc::now),
            updated_at: to_datetime(self.updated_at).unwrap_or_else(Utc::now),
        }
    }
}
