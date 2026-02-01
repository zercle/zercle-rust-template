use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::internal::domain::error::DomainError;
use crate::internal::domain::user::entity::RefreshToken;
use crate::internal::domain::user::traits::{RefreshTokenRepository as RefreshTokenRepositoryTrait};

/// Refresh token repository implementation using SQLx PostgreSQL
pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    /// Create a new RefreshTokenRepository instance
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
impl RefreshTokenRepositoryTrait for RefreshTokenRepository {
    /// Create a new refresh token in the database
    async fn create(&self, token: &RefreshToken) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            from_uuid(token.id),
            from_uuid(token.user_id),
            token.token,
            from_datetime(token.expires_at),
            from_datetime(token.created_at)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get refresh token by token string from the database
    async fn get_by_token(&self, token: &str) -> Result<RefreshToken, DomainError> {
        let row = sqlx::query_as!(
            DbRefreshToken,
            r#"
            SELECT id, user_id, token, expires_at, created_at
            FROM refresh_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                DomainError::TokenInvalid
            } else {
                DomainError::Database(e.to_string())
            }
        })?;

        Ok(row.into_domain())
    }

    /// Delete all refresh tokens for a user
    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE user_id = $1
            "#,
            from_uuid(user_id)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete refresh token by token string
    async fn delete_by_token(&self, token: &str) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE token = $1
            "#,
            token
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::TokenInvalid);
        }

        Ok(())
    }

    /// Delete all expired tokens and return count deleted
    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE expires_at < NOW()
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

/// Internal database refresh token struct for SQLx mapping
struct DbRefreshToken {
    id: sqlx::types::Uuid,
    user_id: sqlx::types::Uuid,
    token: String,
    expires_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::FixedOffset>,
}

impl DbRefreshToken {
    /// Convert database row to domain RefreshToken entity
    fn into_domain(self) -> RefreshToken {
        RefreshToken {
            id: to_uuid(self.id),
            user_id: to_uuid(self.user_id),
            token: self.token,
            expires_at: to_datetime(self.expires_at),
            created_at: to_datetime(self.created_at),
        }
    }
}
