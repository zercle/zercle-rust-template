//! Summary wire types (Go `contract/summary.go` parity, port-spec §5.4b).

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for `GET /reports/summary` (Go `SummaryRequest`) with Go
/// tag `top:"omitempty,min=1"`: absent lets the configured default apply.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
pub struct SummaryRequest {
    #[validate(range(min = 1))]
    pub top: Option<i32>,
}

/// Catalog aggregate (Go `CatalogStats`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CatalogStats {
    pub product_count: i64,
    pub total_stock: i64,
}

/// Machine aggregate (Go `MachineStats`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MachineStats {
    pub machine_count: i64,
    pub total_coin_bank_cents: i64,
}

/// Sales aggregate (Go `SalesStats`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SalesStats {
    pub purchase_count: i64,
    pub revenue_cents: i64,
}

/// One leaderboard row (Go `MachineSales`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MachineSales {
    pub machine_id: String,
    pub label: String,
    pub purchase_count: i64,
    pub revenue_cents: i64,
}

/// Response body for `GET /reports/summary` (Go `SummaryResponse`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SummaryResponse {
    pub catalog: CatalogStats,
    pub machines: MachineStats,
    pub sales: SalesStats,
    pub top_machines: Vec<MachineSales>,
}
