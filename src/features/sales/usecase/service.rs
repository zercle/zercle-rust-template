//! Sales inbound use-case port (Go `usecase.Service` parity, port-spec §5.3d).
//! Allowed dependencies (`tests/architecture.rs`):
//! usecase-depends-on-domain-repository-contract.

use std::sync::Arc;

use async_trait::async_trait;

use crate::features::sales::contract::{PurchaseRequest, PurchaseResponse};
use crate::features::sales::domain::Error;

/// Inbound use-case port for sales.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn purchase(&self, req: PurchaseRequest) -> Result<PurchaseResponse, Error>;
}

/// `Arc`-shared service handle used by handlers and `di`.
pub type SharedService = Arc<dyn Service>;
