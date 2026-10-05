//! Reporting inbound use-case port (Go `usecase.Service` parity, port-spec
//! §5.4d). Allowed dependencies (`tests/architecture.rs`):
//! usecase-depends-on-domain-repository-contract.

use std::sync::Arc;

use async_trait::async_trait;

use crate::features::reporting::contract::{SummaryRequest, SummaryResponse};
use crate::features::reporting::domain::Error;

/// Inbound use-case port for reporting.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn summary(&self, req: SummaryRequest) -> Result<SummaryResponse, Error>;
}

/// `Arc`-shared service handle used by handlers and `di`.
pub type SharedService = Arc<dyn Service>;
