use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Task priority enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Medium => write!(f, "medium"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Urgent => write!(f, "urgent"),
        }
    }
}

/// Task domain entity
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Create a new task with default values
    pub fn new(
        id: Uuid,
        user_id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        priority: TaskPriority,
        due_date: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            title,
            description,
            status,
            priority,
            due_date,
            completed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}
