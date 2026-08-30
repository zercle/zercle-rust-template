//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Inbound use-case port for Items: it speaks the feature's contract types at
//! the boundary so driving adapters bind responses directly and never map to
//! or from domain entities (Go `application/service.go` parity).

use std::sync::Arc;

use async_trait::async_trait;

use crate::features::example::contract::{
    CreateItemRequest, ItemResponse, ListItemsRequest, ListItemsResponse,
};
use crate::features::example::domain::Error;

/// Inbound use-case port for `Item`.
///
/// Driving adapters (`adapter/in`) call this; the only permitted dependencies
/// are this feature's domain, port, and contract
/// (`tests/architecture.rs`: application-depends-on-domain-port-contract).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn create(&self, req: CreateItemRequest) -> Result<ItemResponse, Error>;
    async fn get(&self, id: String) -> Result<ItemResponse, Error>;
    async fn list(&self, req: ListItemsRequest) -> Result<ListItemsResponse, Error>;
}

/// Type alias for an `Arc`-shared service handle.
pub type SharedService = Arc<dyn Service>;
