//! Catalog inbound use-case port (Go `usecase.Service` parity, port-spec
//! §5.1d). It speaks the feature's contract types at the boundary so the
//! handler binds responses directly and never map to/from domain entities.
//!
//! Allowed dependencies (`tests/architecture.rs`:
//! usecase-depends-on-domain-repository-contract): this feature's domain, port,
//! and contract only.

use std::sync::Arc;

use async_trait::async_trait;

use crate::features::catalog::contract::{
    CreateProductRequest, ListProductsRequest, ListProductsResponse, ProductResponse,
};
use crate::features::catalog::domain::Error;

/// Inbound use-case port for catalog products.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn create(&self, req: CreateProductRequest) -> Result<ProductResponse, Error>;
    async fn get(&self, id: String) -> Result<ProductResponse, Error>;
    async fn list(&self, req: ListProductsRequest) -> Result<ListProductsResponse, Error>;
}

/// `Arc`-shared service handle used by handlers and `di`.
pub type SharedService = Arc<dyn Service>;
