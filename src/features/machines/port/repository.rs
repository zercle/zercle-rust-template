//! Machines repository port (Go `repository.Repository` parity, port-spec
//! §5.2e). May reference only this feature's domain
//! (`tests/architecture.rs`: port-depends-only-on-domain).

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::machines::domain::{Error, Machine};

/// Outbound persistence port for machines.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn create(&self, machine: &Machine) -> Result<(), Error>;
    async fn get_by_id(&self, id: Uuid) -> Result<Machine, Error>;
    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Machine>, Error>;
    /// Add coins to the machine's bank in one transaction (row-locked), then
    /// the usecase re-reads for the authoritative bank. A missing machine maps
    /// to `Error::MachineNotFound`.
    async fn restock_bank(&self, id: Uuid, coins: &[i32]) -> Result<(), Error>;
}
