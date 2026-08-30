//! STUB FEATURE — delete src/features/example to start your project.

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::example::domain::{Error, Item};

/// Outbound persistence port for `Item` (Go `port.Repository` parity).
///
/// The application layer depends on this abstraction; persistence adapters
/// under `adapter/out` satisfy it. May reference only this feature's domain
/// (`tests/architecture.rs`: port-depends-only-on-domain).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn create(&self, item: &Item) -> Result<(), Error>;
    async fn get_by_id(&self, id: Uuid) -> Result<Item, Error>;
    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Item>, Error>;
}
