//! Repository module for database access layer
//!
//! This module contains all repository implementations using SQLx PostgreSQL.
//! Each repository handles CRUD operations for its respective domain entity.

pub mod task;
pub mod user;
pub mod refresh_token;

pub use task::{TaskRepository, TaskRepositoryTrait};
pub use user::{UserRepository, UserRepositoryTrait};
pub use refresh_token::{RefreshTokenRepository, RefreshTokenRepositoryTrait};

use sqlx::postgres::PgPool;
use uuid::Uuid;

/// Factory function to create a UserRepository
pub fn new_user_repository(pool: PgPool) -> UserRepository {
    UserRepository::new(pool)
}

/// Factory function to create a RefreshTokenRepository
pub fn new_refresh_token_repository(pool: PgPool) -> RefreshTokenRepository {
    RefreshTokenRepository::new(pool)
}

/// Factory function to create a TaskRepository
pub fn new_task_repository(pool: PgPool) -> TaskRepository {
    TaskRepository::new(pool)
}
