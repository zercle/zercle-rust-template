//! Catalog use-case implementation (Go `usecase/usecase.go` parity, port-spec
//! §5.1d). Business rules:
//!
//! * name is trimmed; empty or `> max_name_length` runes → `InvalidProductName`
//! * `price_cents <= 0` → `InvalidPrice`
//! * stock is not re-validated here (contract `gte=0` + DB CHECK)
//! * timestamps are `now` (UTC)
//! * pagination: `limit<=0 -> default`, `limit>max -> max`, `offset<0 -> 0`
//!
//! Config values `<= 0` fall back to the package defaults (20 / 100 / 255),
//! mirroring Go.

use std::sync::Arc;

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::features::catalog::contract::{
    CreateProductRequest, ListProductsRequest, ListProductsResponse, ProductResponse,
};
use crate::features::catalog::domain::{Error, Product};
use crate::features::catalog::repository::Repository;
use crate::features::catalog::usecase::Service;

const DEFAULT_PAGE_SIZE: i32 = 20;
const MAX_PAGE_SIZE: i32 = 100;
const MAX_NAME_LENGTH: usize = 255;

/// Concrete catalog use case backed by a [`Repository`] outbound interface.
#[derive(Clone)]
pub struct Usecase {
    repo: Arc<dyn Repository>,
    default_page_size: i32,
    max_page_size: i32,
    max_name_length: usize,
}

impl Usecase {
    /// Build a use case. Values `<= 0` fall back to 20 / 100 / 255.
    pub fn new(
        repo: Arc<dyn Repository>,
        default_page_size: i32,
        max_page_size: i32,
        max_name_length: i32,
    ) -> Self {
        Self {
            repo,
            default_page_size: positive_or(default_page_size, DEFAULT_PAGE_SIZE),
            max_page_size: positive_or(max_page_size, MAX_PAGE_SIZE),
            max_name_length: positive_or(max_name_length, MAX_NAME_LENGTH as i32) as usize,
        }
    }

    /// Map a domain product to its wire form (RFC 3339 timestamps).
    fn product_response(product: &Product) -> ProductResponse {
        ProductResponse {
            id: product.id.to_string(),
            name: product.name.clone(),
            price_cents: product.price_cents,
            stock: product.stock,
            created_at: format_rfc3339(product.created_at),
            updated_at: format_rfc3339(product.updated_at),
        }
    }
}

#[async_trait::async_trait]
impl Service for Usecase {
    async fn create(&self, req: CreateProductRequest) -> Result<ProductResponse, Error> {
        let name = req.name.trim();
        // Rune count, not UTF-8 byte length (Go `utf8.RuneCountInString`).
        if name.is_empty() || name.chars().count() > self.max_name_length {
            return Err(Error::InvalidProductName);
        }
        if req.price_cents <= 0 {
            return Err(Error::InvalidPrice);
        }
        let now = OffsetDateTime::now_utc();
        let product = Product {
            id: Uuid::now_v7(),
            name: name.to_string(),
            price_cents: req.price_cents,
            stock: req.stock,
            created_at: now,
            updated_at: now,
        };
        self.repo.create(&product).await?;
        Ok(Self::product_response(&product))
    }

    async fn get(&self, id: String) -> Result<ProductResponse, Error> {
        // One id-parsing path shared by both future handlers.
        let id = Uuid::parse_str(&id).map_err(|_| Error::InvalidId)?;
        let product = self.repo.get_by_id(id).await?;
        Ok(Self::product_response(&product))
    }

    async fn list(&self, req: ListProductsRequest) -> Result<ListProductsResponse, Error> {
        let mut limit = req.limit.unwrap_or(0);
        if limit <= 0 {
            limit = self.default_page_size;
        }
        if limit > self.max_page_size {
            limit = self.max_page_size;
        }
        let offset = req.offset.unwrap_or(0).max(0);
        let products = self.repo.list(limit, offset).await?;
        Ok(ListProductsResponse {
            products: products.iter().map(Self::product_response).collect(),
        })
    }
}

fn positive_or(value: i32, fallback: i32) -> i32 {
    if value <= 0 { fallback } else { value }
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::catalog::repository::MockRepository;
    use mockall::predicate::*;

    fn product(id: Uuid, name: &str, price_cents: i32) -> Product {
        let now = OffsetDateTime::now_utc();
        Product {
            id,
            name: name.to_string(),
            price_cents,
            stock: 5,
            created_at: now,
            updated_at: now,
        }
    }

    fn usecase(repo: MockRepository) -> Usecase {
        Usecase::new(Arc::new(repo), 20, 100, 255)
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateProductRequest {
                name: "   ".to_string(),
                price_cents: 10,
                stock: 1,
            })
            .await
            .unwrap_err(),
            Error::InvalidProductName
        );
    }

    #[tokio::test]
    async fn create_rejects_overlong_name() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateProductRequest {
                name: "a".repeat(256),
                price_cents: 10,
                stock: 1,
            })
            .await
            .unwrap_err(),
            Error::InvalidProductName
        );
    }

    #[tokio::test]
    async fn create_accepts_multibyte_name_within_rune_cap() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = usecase(mock);
        let name = "ช".repeat(200);
        let resp = svc
            .create(CreateProductRequest {
                name,
                price_cents: 10,
                stock: 1,
            })
            .await
            .unwrap();
        assert_eq!(resp.name.chars().count(), 200);
    }

    #[tokio::test]
    async fn create_rejects_multibyte_name_over_rune_cap() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateProductRequest {
                name: "🎉".repeat(256),
                price_cents: 10,
                stock: 1,
            })
            .await
            .unwrap_err(),
            Error::InvalidProductName
        );
    }

    #[tokio::test]
    async fn create_rejects_non_positive_price() {
        let svc = usecase(MockRepository::new());
        for price in [0, -1] {
            assert_eq!(
                svc.create(CreateProductRequest {
                    name: "cola".to_string(),
                    price_cents: price,
                    stock: 1,
                })
                .await
                .unwrap_err(),
                Error::InvalidPrice
            );
        }
    }

    #[tokio::test]
    async fn create_trims_and_persists() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = usecase(mock);
        let resp = svc
            .create(CreateProductRequest {
                name: "  cola  ".to_string(),
                price_cents: 150,
                stock: 2,
            })
            .await
            .unwrap();
        assert_eq!(resp.name, "cola");
        assert_eq!(resp.price_cents, 150);
    }

    #[tokio::test]
    async fn create_maps_domain_to_wire_timestamps() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = usecase(mock);
        let resp = svc
            .create(CreateProductRequest {
                name: "alpha".to_string(),
                price_cents: 10,
                stock: 0,
            })
            .await
            .unwrap();
        assert!(resp.created_at.ends_with('Z'), "got {}", resp.created_at);
        assert!(Uuid::parse_str(&resp.id).is_ok());
    }

    #[tokio::test]
    async fn get_rejects_malformed_id_before_touching_the_port() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id().times(0);
        let svc = usecase(mock);
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
            .returning(|_| Err(Error::ProductNotFound));
        let svc = usecase(mock);
        assert_eq!(
            svc.get(Uuid::nil().to_string()).await.unwrap_err(),
            Error::ProductNotFound
        );
    }

    #[tokio::test]
    async fn get_returns_wire_response_on_hit() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|id| Ok(product(id, "x", 42)));
        let svc = usecase(mock);
        let got = svc.get(Uuid::nil().to_string()).await.unwrap();
        assert_eq!(got.name, "x");
        assert_eq!(got.price_cents, 42);
        assert_eq!(got.id, Uuid::nil().to_string());
    }

    #[tokio::test]
    async fn list_clamps_limit_above_max() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = usecase(mock);
        let resp = svc
            .list(ListProductsRequest {
                limit: Some(9_999),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(resp.products.is_empty());
    }

    #[tokio::test]
    async fn list_uses_default_when_limit_zero_or_missing() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 20 && *offset == 0)
            .returning(|_, _| Ok(vec![]))
            .times(2);
        let svc = usecase(mock);
        svc.list(ListProductsRequest {
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
        svc.list(ListProductsRequest {
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
        let svc = usecase(mock);
        svc.list(ListProductsRequest {
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
        let svc = Usecase::new(Arc::new(mock), 0, 0, 0);
        svc.list(ListProductsRequest {
            limit: Some(9_999),
            offset: Some(0),
        })
        .await
        .unwrap();
    }
}
