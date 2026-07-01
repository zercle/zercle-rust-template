//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Domain types + traits for the example feature (canonical clean-architecture
//! demo). Mirrors Go `internal/features/example/domain/{item,errors,repository,service}.go`.
//!
//! Per decision-log D6, traits take no explicit context parameter — the impl
//! holds the pool and emits request-scoped spans via `tracing`.

use std::sync::Arc;

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::errors::AppError;

/// The trivial example entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Item {
    /// Replace the name and refresh the `updated_at` timestamp to now (UTC).
    pub fn rename(&mut self, name: String) {
        self.name = name;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

/// Domain sentinel errors. Mapping to the shared boundary `AppError` lives in
/// the `From` impl below so `shared` does not import this feature (D7).
///
/// `Internal` carries infrastructure failures (`sqlx`, etc.) that don't map to
/// a semantic sentinel; it parallels Go's `fmt.Errorf("...: %w", err)` wrappers
/// in `service.go`. The three semantic sentinels are mapped to `AppError`; the
/// `Internal` variant forwards the cause.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("item not found")]
    NotFound,
    #[error("item name is invalid")]
    InvalidName,
    #[error("item id is invalid")]
    InvalidId,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::NotFound, Error::NotFound)
            | (Error::InvalidName, Error::InvalidName)
            | (Error::InvalidId, Error::InvalidId) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotFound => AppError::NotFound { cause: None },
            Error::InvalidName | Error::InvalidId => AppError::InvalidInput { cause: None },
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Outbound persistence port for `Item`.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn create(&self, item: &Item) -> Result<(), Error>;
    async fn get_by_id(&self, id: Uuid) -> Result<Item, Error>;
    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Item>, Error>;
}

/// Inbound use-case port for `Item`.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn create(&self, name: String) -> Result<Item, Error>;
    async fn get(&self, id: Uuid) -> Result<Item, Error>;
    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Item>, Error>;
}

/// Type alias for an `Arc`-shared service handle.
pub type SharedService = Arc<dyn Service>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_updates_name_and_timestamp() {
        let mut item = Item {
            id: Uuid::nil(),
            name: "old".to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        item.rename("new".to_string());
        assert_eq!(item.name, "new");
        assert!(item.updated_at > OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn error_maps_to_app_error() {
        assert!(matches!(
            AppError::from(Error::NotFound),
            AppError::NotFound { .. }
        ));
        assert!(matches!(
            AppError::from(Error::InvalidName),
            AppError::InvalidInput { .. }
        ));
        assert!(matches!(
            AppError::from(Error::InvalidId),
            AppError::InvalidInput { .. }
        ));
    }
}
