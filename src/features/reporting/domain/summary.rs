//! Reporting read-model entities (Go `domain/summary.go` parity, port-spec
//! §5.4c).

use uuid::Uuid;

/// Whole-database overview (Go `domain.Overview`). `total_bank_cents` maps to
/// contract `total_coin_bank_cents`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overview {
    pub product_count: i64,
    pub total_stock: i64,
    pub machine_count: i64,
    pub total_bank_cents: i64,
    pub purchase_count: i64,
    pub revenue_cents: i64,
}

/// One machine's sales rollup (Go `domain.MachineSales`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSales {
    pub machine_id: Uuid,
    pub label: String,
    pub purchase_count: i64,
    pub revenue_cents: i64,
}
