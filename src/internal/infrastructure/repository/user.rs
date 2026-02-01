use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::user::entity::User;
use crate::internal::domain::user::traits::{UserRepository as UserRepositoryTrait};

/// User repository implementation using SQLx PostgreSQL
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Create a new UserRepository instance
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
fn to_datetime(ts: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>) -> DateTime<Utc> {
    ts.with_timezone(&Utc)
}

/// Helper function to convert domain DateTime<Utc> to SQLx DateTime
fn from_datetime(dt: DateTime<Utc>) -> sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset> {
    dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    /// Create a new user in the database
    async fn create(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password_hash, full_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            from_uuid(user.id),
            user.email,
            user.password_hash,
            user.full_name,
            from_datetime(user.created_at),
            from_datetime(user.updated_at)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return DomainError::EmailAlreadyExists;
                }
            }
            DomainError::Database(e.to_string())
        })?;

        Ok(())
    }

    /// Get user by ID from the database
    async fn get_by_id(&self, id: Uuid) -> Result<User, DomainError> {
        let row = sqlx::query_as!(
            DbUser,
            r#"
            SELECT id, email, password_hash, full_name, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            from_uuid(id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                DomainError::UserNotFound
            } else {
                DomainError::Database(e.to_string())
            }
        })?;

        Ok(row.into_domain())
    }

    /// Get user by email from the database
    async fn get_by_email(&self, email: &str) -> Result<User, DomainError> {
        let row = sqlx::query_as!(
            DbUser,
            r#"
            SELECT id, email, password_hash, full_name, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                DomainError::UserNotFound
            } else {
                DomainError::Database(e.to_string())
            }
        })?;

        Ok(row.into_domain())
    }

    /// Update user in the database
    async fn update(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET email = $1, password_hash = $2, full_name = $3, updated_at = $4
            WHERE id = $5
            "#,
            user.email,
            user.password_hash,
            user.full_name,
            from_datetime(user.updated_at),
            from_uuid(user.id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete user by ID from the database
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            from_uuid(id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }

        Ok(())
    }

    /// Check if user exists by email
    async fn exists_by_email(&self, email: &str) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            SELECT 1 FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(result.is_some())
    }
}

/// Internal database user struct for SQLx mapping
struct DbUser {
    id: sqlx::types::Uuid,
    email: String,
    password_hash: String,
    full_name: Option<String>,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
}

impl DbUser {
    /// Convert database row to domain User entity
    fn into_domain(self) -> User {
        User {
            id: to_uuid(self.id),
            email: self.email,
            password_hash: self.password_hash,
            full_name: self.full_name,
            created_at: to_datetime(self.created_at),
            updated_at: to_datetime(self.updated_at),
        }
    }
}
