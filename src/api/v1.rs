//! Published inbound contract of the `/api/v1` endpoints (Go `pkg/api/v1`
//! parity): the request/response wire types of the four demo features plus the
//! error codes other services may import.
//!
//! Facade of the canonical types in each owning feature's `contract` module —
//! internal code must not import this module
//! (`tests/architecture.rs`: published-contract-is-outward-only). A future v2
//! contract is a new facade module (`api::v2`), not a change here.

pub use crate::features::catalog::contract::{
    CreateProductRequest, ListProductsRequest, ListProductsResponse, ProductResponse,
};
pub use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, ListMachinesResponse, MachineResponse,
    RestockBankRequest,
};
pub use crate::features::reporting::contract::{
    CatalogStats, MachineSales, MachineStats, SalesStats, SummaryRequest, SummaryResponse,
};
pub use crate::features::sales::contract::{PurchaseRequest, PurchaseResponse};

/// Error codes carried in the JSON error envelope (`{"error": CODE, ...}`).
pub use crate::platform::errors::errcodes;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_aliases_round_trip_json() {
        let req = CreateProductRequest {
            name: "from-a-consumer".to_string(),
            price_cents: 150,
            stock: 2,
        };
        let data = serde_json::to_string(&req).unwrap();
        assert_eq!(
            data,
            r#"{"name":"from-a-consumer","price_cents":150,"stock":2}"#
        );

        let resp = PurchaseResponse {
            id: "id".to_string(),
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            price_cents: 150,
            total_inserted_cents: 175,
            change_cents: 25,
            change_coins: vec![25],
            purchased_at: "t".to_string(),
        };
        let data = serde_json::to_string(&resp).unwrap();
        assert!(data.contains(r#""change_coins":[25]"#), "got {data}");
    }

    #[test]
    fn errcode_re_exports() {
        use errcodes::*;
        assert_eq!(NOT_FOUND, "NOT_FOUND");
        assert_eq!(INVALID_INPUT, "INVALID_INPUT");
        assert_eq!(INTERNAL, "INTERNAL");
    }
}
