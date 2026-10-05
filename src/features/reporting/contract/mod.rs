//! reporting contract — canonical inbound wire type for `/api/v1/reports/summary`
//! (Go `internal/features/reporting/contract` parity, port-spec §5.4b).
//!
//! Leaf rule: no crate-internal imports (`tests/architecture.rs`:
//! contract-is-leaf).

pub mod summary;

pub use summary::{
    CatalogStats, MachineSales, MachineStats, SalesStats, SummaryRequest, SummaryResponse,
};

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn summary_top_is_optional_but_positive_when_present() {
        let absent = SummaryRequest { top: None };
        assert!(
            absent.validate().is_ok(),
            "omitempty lets the default apply"
        );
        let zero = SummaryRequest { top: Some(0) };
        assert!(zero.validate().is_err());
        let ok = SummaryRequest { top: Some(1) };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn summary_response_uses_go_json_names() {
        let resp = SummaryResponse {
            catalog: CatalogStats {
                product_count: 1,
                total_stock: 2,
            },
            machines: MachineStats {
                machine_count: 3,
                total_coin_bank_cents: 4,
            },
            sales: SalesStats {
                purchase_count: 5,
                revenue_cents: 6,
            },
            top_machines: vec![MachineSales {
                machine_id: "m".to_string(),
                label: "lobby".to_string(),
                purchase_count: 7,
                revenue_cents: 8,
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""total_coin_bank_cents":4"#), "got {json}");
        assert!(
            json.contains(r#""top_machines":[{"machine_id":"m""#),
            "got {json}"
        );
    }
}
