//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Implementation of `domain::Service`. Mirrors Go
//! `internal/features/example/service/service.go`.

use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::example::domain::{Error, Item, Repository, Service};

const DEFAULT_PAGE_SIZE: i32 = 20;
const MAX_PAGE_SIZE: i32 = 100;
const MAX_NAME_LENGTH: usize = 255;

/// Concrete use-case service backed by a `domain::Repository`.
#[derive(Clone)]
pub struct ServiceImpl {
    repo: Arc<dyn Repository>,
    default_page_size: i32,
    max_page_size: i32,
    max_name_length: usize,
}

impl ServiceImpl {
    /// Build a service. Values `<= 0` fall back to the package defaults
    /// (`20` / `100` / `255`), mirroring Go.
    pub fn new(
        repo: Arc<dyn Repository>,
        default_page_size: i32,
        max_page_size: i32,
        max_name_length: i32,
    ) -> Self {
        let default_page_size = if default_page_size <= 0 {
            DEFAULT_PAGE_SIZE
        } else {
            default_page_size
        };
        let max_page_size = if max_page_size <= 0 {
            MAX_PAGE_SIZE
        } else {
            max_page_size
        };
        let max_name_length = if max_name_length <= 0 {
            MAX_NAME_LENGTH
        } else {
            max_name_length as usize
        };
        Self {
            repo,
            default_page_size,
            max_page_size,
            max_name_length,
        }
    }
}

#[async_trait::async_trait]
impl Service for ServiceImpl {
    async fn create(&self, name: String) -> Result<Item, Error> {
        let name = name.trim();
        if name.is_empty() || name.len() > self.max_name_length {
            return Err(Error::InvalidName);
        }
        let now = OffsetDateTime::now_utc();
        let item = Item {
            id: Uuid::now_v7(),
            name: name.to_string(),
            created_at: now,
            updated_at: now,
        };
        self.repo.create(&item).await?;
        Ok(item)
    }

    async fn get(&self, id: Uuid) -> Result<Item, Error> {
        self.repo.get_by_id(id).await
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Item>, Error> {
        let mut limit = if limit <= 0 {
            self.default_page_size
        } else {
            limit
        };
        if limit > self.max_page_size {
            limit = self.max_page_size;
        }
        let offset = if offset < 0 { 0 } else { offset };
        self.repo.list(limit, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::example::domain::MockRepository;
    use mockall::predicate::*;

    fn item(id: Uuid, name: &str) -> Item {
        let now = OffsetDateTime::now_utc();
        Item {
            id,
            name: name.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let repo = Arc::new(MockRepository::new());
        let svc = ServiceImpl::new(repo, 20, 100, 255);
        assert_eq!(
            svc.create("   ".to_string()).await.unwrap_err(),
            Error::InvalidName
        );
    }

    #[tokio::test]
    async fn create_rejects_overlong_name() {
        let repo = Arc::new(MockRepository::new());
        let svc = ServiceImpl::new(repo, 20, 100, 255);
        let big = "a".repeat(256);
        assert_eq!(svc.create(big).await.unwrap_err(), Error::InvalidName);
    }

    #[tokio::test]
    async fn create_trims_and_persists() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let repo = Arc::new(mock);
        let svc = ServiceImpl::new(repo, 20, 100, 255);
        let it = svc.create("  hello  ".to_string()).await.unwrap();
        assert_eq!(it.name, "hello");
        assert_eq!(it.created_at, it.updated_at);
    }

    #[tokio::test]
    async fn get_passes_through_not_found() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|_| Err(Error::NotFound));
        let svc = ServiceImpl::new(Arc::new(mock), 20, 100, 255);
        assert_eq!(svc.get(Uuid::nil()).await.unwrap_err(), Error::NotFound);
    }

    #[tokio::test]
    async fn get_returns_item_on_hit() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|_| Ok(item(Uuid::nil(), "x")));
        let svc = ServiceImpl::new(Arc::new(mock), 20, 100, 255);
        let got = svc.get(Uuid::nil()).await.unwrap();
        assert_eq!(got.name, "x");
    }

    #[tokio::test]
    async fn list_clamps_limit_above_max() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = ServiceImpl::new(Arc::new(mock), 20, 100, 255);
        let items = svc.list(9_999, 0).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn list_uses_default_when_limit_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 20 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = ServiceImpl::new(Arc::new(mock), 20, 100, 255);
        svc.list(0, 0).await.unwrap();
    }

    #[tokio::test]
    async fn list_clamps_negative_offset_to_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 10 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = ServiceImpl::new(Arc::new(mock), 20, 100, 255);
        svc.list(10, -5).await.unwrap();
    }

    #[tokio::test]
    async fn fallback_defaults_apply_when_config_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        // All config values ≤ 0 → fall back to 20/100/255
        let svc = ServiceImpl::new(Arc::new(mock), 0, 0, 0);
        svc.list(9_999, 0).await.unwrap();
    }
}
