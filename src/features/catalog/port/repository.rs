//! Catalog repository port (Go `repository.Repository` parity, port-spec
//! §5.1e). The application layer depends on this abstraction; the postgres
//! adapter (next wave) satisfies it.
//!
//! May reference only this feature's domain (`tests/architecture.rs`:
//! port-depends-only-on-domain).

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::catalog::domain::{Error, Product};

/// Outbound persistence port for products.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn create(&self, product: &Product) -> Result<(), Error>;
    async fn get_by_id(&self, id: Uuid) -> Result<Product, Error>;
    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Product>, Error>;
}
