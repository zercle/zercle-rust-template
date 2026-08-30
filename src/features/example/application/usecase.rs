//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Implementation of the [`Service`](super::Service) inbound port (Go
//! `application/usecase.go` parity). Orchestrates the domain and the outbound
//! port, and owns the domain ↔ contract mapping so driving adapters never see
//! domain entities.

use std::sync::Arc;

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::features::example::application::Service;
use crate::features::example::contract::{
    CreateItemRequest, ItemResponse, ListItemsRequest, ListItemsResponse,
};
use crate::features::example::domain::{Error, Item};
use crate::features::example::port::Repository;

const DEFAULT_PAGE_SIZE: i32 = 20;
const MAX_PAGE_SIZE: i32 = 100;
const MAX_NAME_LENGTH: usize = 255;

/// Concrete use case backed by a [`Repository`] outbound port.
#[derive(Clone)]
pub struct Usecase {
    repo: Arc<dyn Repository>,
    default_page_size: i32,
    max_page_size: i32,
    max_name_length: usize,
}

impl Usecase {
    /// Build a use case. Values `<= 0` fall back to the package defaults
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

    /// Map a domain item to its wire form (RFC 3339 timestamps).
    fn item_response(item: &Item) -> ItemResponse {
        ItemResponse {
            id: item.id.to_string(),
            name: item.name.clone(),
            created_at: format_rfc3339(item.created_at),
            updated_at: format_rfc3339(item.updated_at),
        }
    }
}

#[async_trait::async_trait]
impl Service for Usecase {
    async fn create(&self, req: CreateItemRequest) -> Result<ItemResponse, Error> {
        let name = req.name.trim();
        // Mirror Go's `utf8.RuneCountInString(name) > maxNameLength`: count
        // Unicode scalar values (chars), not UTF-8 bytes, so multi-byte
        // names (e.g. CJK, Thai, emoji) follow the documented 255-rune cap
        // rather than failing on raw byte length.
        if name.is_empty() || name.chars().count() > self.max_name_length {
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
        Ok(Self::item_response(&item))
    }

    async fn get(&self, id: String) -> Result<ItemResponse, Error> {
        // The wire id string is parsed here so both driving adapters share one
        // validation path (Go usecase.Get parity).
        let id = Uuid::parse_str(&id).map_err(|_| Error::InvalidId)?;
        let item = self.repo.get_by_id(id).await?;
        Ok(Self::item_response(&item))
    }

    async fn list(&self, req: ListItemsRequest) -> Result<ListItemsResponse, Error> {
        let mut limit = req.limit.unwrap_or(0);
        if limit <= 0 {
            limit = self.default_page_size;
        }
        if limit > self.max_page_size {
            limit = self.max_page_size;
        }
        let offset = req.offset.unwrap_or(0);
        let offset = if offset < 0 { 0 } else { offset };
        let items = self.repo.list(limit, offset).await?;
        Ok(ListItemsResponse {
            items: items.iter().map(Self::item_response).collect(),
        })
    }
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::example::port::MockRepository;
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
        let svc = Usecase::new(repo, 20, 100, 255);
        assert_eq!(
            svc.create(CreateItemRequest {
                name: "   ".to_string(),
            })
            .await
            .unwrap_err(),
            Error::InvalidName
        );
    }

    #[tokio::test]
    async fn create_rejects_overlong_name() {
        let repo = Arc::new(MockRepository::new());
        let svc = Usecase::new(repo, 20, 100, 255);
        let big = "a".repeat(256);
        assert_eq!(
            svc.create(CreateItemRequest { name: big })
                .await
                .unwrap_err(),
            Error::InvalidName
        );
    }

    #[tokio::test]
    async fn create_accepts_multibyte_name_within_rune_cap() {
        // 200 Thai "ช" (U+0E0A, 3 UTF-8 bytes each = 600 bytes) would be
        // rejected by a byte-length check, but must pass under rune-count
        // matching Go's `utf8.RuneCountInString`. Cap is 255 runes.
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        let name = "ช".repeat(200);
        assert_eq!(name.len(), 600, "sanity: 3 bytes per Thai char");
        let resp = svc.create(CreateItemRequest { name }).await.unwrap();
        assert_eq!(resp.name.chars().count(), 200);
    }

    #[tokio::test]
    async fn create_rejects_multibyte_name_over_rune_cap() {
        // 256 emoji 🎉 (U+1F389, 4 UTF-8 bytes each = 1024 bytes) — must
        // be rejected because 256 > 255 rune cap (the 1024-byte-count
        // version would also reject, but the rune-count version is the
        // parity contract with Go).
        let repo = Arc::new(MockRepository::new());
        let svc = Usecase::new(repo, 20, 100, 255);
        let name = "🎉".repeat(256);
        assert_eq!(name.len(), 1024, "sanity: 4 bytes per emoji");
        assert_eq!(
            svc.create(CreateItemRequest { name }).await.unwrap_err(),
            Error::InvalidName
        );
    }

    #[tokio::test]
    async fn create_trims_and_persists() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let repo = Arc::new(mock);
        let svc = Usecase::new(repo, 20, 100, 255);
        let resp = svc
            .create(CreateItemRequest {
                name: "  hello  ".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(resp.name, "hello");
    }

    #[tokio::test]
    async fn get_rejects_malformed_id_before_touching_the_port() {
        // One validation path for both driving adapters: a bad uuid fails
        // without a repository call.
        let mut mock = MockRepository::new();
        mock.expect_get_by_id().times(0);
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        assert_eq!(
            svc.get("not-a-uuid".to_string()).await.unwrap_err(),
            Error::InvalidId
        );
    }

    #[tokio::test]
    async fn get_passes_through_not_found() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|_| Err(Error::NotFound));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        assert_eq!(
            svc.get(Uuid::nil().to_string()).await.unwrap_err(),
            Error::NotFound
        );
    }

    #[tokio::test]
    async fn get_returns_wire_response_on_hit() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|id| Ok(item(id, "x")));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        let got = svc.get(Uuid::nil().to_string()).await.unwrap();
        assert_eq!(got.name, "x");
        assert_eq!(got.id, Uuid::nil().to_string());
    }

    #[tokio::test]
    async fn list_clamps_limit_above_max() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        let resp = svc
            .list(ListItemsRequest {
                limit: Some(9_999),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(resp.items.is_empty());
    }

    #[tokio::test]
    async fn list_uses_default_when_limit_zero_or_missing() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 20 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        // limit absent entirely (None) and limit = 0 both hit the default.
        svc.list(ListItemsRequest {
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
        svc.list(ListItemsRequest {
            limit: Some(0),
            offset: Some(0),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_clamps_negative_offset_to_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 10 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        svc.list(ListItemsRequest {
            limit: Some(10),
            offset: Some(-5),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn fallback_defaults_apply_when_config_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        // All config values ≤ 0 → fall back to 20/100/255
        let svc = Usecase::new(Arc::new(mock), 0, 0, 0);
        svc.list(ListItemsRequest {
            limit: Some(9_999),
            offset: Some(0),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_maps_domain_item_to_wire_timestamps() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = Usecase::new(Arc::new(mock), 20, 100, 255);
        let resp = svc
            .create(CreateItemRequest {
                name: "alpha".to_string(),
            })
            .await
            .unwrap();
        // RFC 3339 with a Z suffix — exercised lightly; exact formatting is
        // covered by the contract + facade tests.
        assert!(resp.created_at.ends_with('Z'), "got {}", resp.created_at);
    }
}
