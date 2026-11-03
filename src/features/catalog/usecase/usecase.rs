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
mod tests;
