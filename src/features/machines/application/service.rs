//! Machines inbound use-case port (Go `usecase.Service` parity, port-spec
//! §5.2d). Allowed dependencies (`tests/architecture.rs`):
//! application-depends-on-domain-port-contract.

use std::sync::Arc;

use async_trait::async_trait;

use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, ListMachinesResponse, MachineResponse,
    RestockBankRequest,
};
use crate::features::machines::domain::Error;

/// Inbound use-case port for machines.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Service: Send + Sync {
    async fn create(&self, req: CreateMachineRequest) -> Result<MachineResponse, Error>;
    async fn get(&self, id: String) -> Result<MachineResponse, Error>;
    async fn list(&self, req: ListMachinesRequest) -> Result<ListMachinesResponse, Error>;
    async fn restock_bank(
        &self,
        id: String,
        req: RestockBankRequest,
    ) -> Result<MachineResponse, Error>;
}

/// `Arc`-shared service handle used by driving adapters and `di`.
pub type SharedService = Arc<dyn Service>;
