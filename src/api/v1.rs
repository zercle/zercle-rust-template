//! Published inbound contract of the `/api/v1` endpoints (Go `pkg/api/v1`
//! parity): the request/response wire types plus the error codes that other
//! services may import.
//!
//! Facade of the canonical types in the owning feature's `contract` module —
//! internal code must not import this module. A future v2 contract is a new
//! facade module (`api::v2`), not a change here.

pub use crate::features::example::contract::{
    CreateItemRequest, ItemResponse, ListItemsRequest, ListItemsResponse,
};

/// Error codes carried in the JSON error envelope (`{"error": CODE, ...}`).
pub use crate::platform::errors::errcodes;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_aliases_round_trip_json() {
        let req = CreateItemRequest {
            name: "from-a-consumer".to_string(),
        };
        let data = serde_json::to_string(&req).unwrap();
        assert_eq!(data, r#"{"name":"from-a-consumer"}"#);

        let resp = ItemResponse {
            id: "id".to_string(),
            name: "n".to_string(),
            created_at: "t1".to_string(),
            updated_at: "t2".to_string(),
        };
        let data = serde_json::to_string(&ListItemsResponse { items: vec![resp] }).unwrap();
        assert_eq!(
            data,
            r#"{"items":[{"id":"id","name":"n","created_at":"t1","updated_at":"t2"}]}"#
        );
    }

    #[test]
    fn errcode_re_exports() {
        use errcodes::*;
        assert_eq!(NOT_FOUND, "NOT_FOUND");
        assert_eq!(INVALID_INPUT, "INVALID_INPUT");
        assert_eq!(INTERNAL, "INTERNAL");
    }
}
