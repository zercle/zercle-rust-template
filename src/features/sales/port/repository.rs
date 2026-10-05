//! Sales repository port (Go `repository.Repository` parity, port-spec §5.3e).
//! It reads catalog/machines tables through its own port and writes the
//! purchase and its effects in one transaction (deliberate single-database
//! compromise; a distributed deployment would swap this port for
//! cross-service calls/saga).
//!
//! May reference only this feature's domain (`tests/architecture.rs`:
//! port-depends-only-on-domain).

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::sales::domain::{CoinBank, Error, PurchaseRecord, SaleProduct};

/// Outbound persistence port for purchases.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn get_product(&self, product_id: Uuid) -> Result<SaleProduct, Error>;
    async fn get_machine_bank(&self, machine_id: Uuid) -> Result<CoinBank, Error>;
    /// Commit the purchase, decrement stock, and replace the machine bank in
    /// one transaction. A stock race loses with `Error::OutOfStock`.
    async fn commit_purchase(
        &self,
        machine_id: Uuid,
        product_id: Uuid,
        record: &PurchaseRecord,
        bank_after: &CoinBank,
    ) -> Result<(), Error>;
}
