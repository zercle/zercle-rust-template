//! Reporting repository port (Go `repository.Repository` parity, port-spec
//! §5.4e). Read-only; it reads catalog_products, machines, and sales_purchases
//! through its own port (deliberate single-database compromise).
//!
//! May reference only this feature's domain (`tests/architecture.rs`:
//! port-depends-only-on-domain).

use async_trait::async_trait;

use crate::features::reporting::domain::{Error, MachineSales, Overview};

/// Outbound read-only aggregation port.
#[allow(clippy::double_must_use)]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn get_overview(&self) -> Result<Overview, Error>;
    async fn get_top_machines(&self, limit: i32) -> Result<Vec<MachineSales>, Error>;
}
