//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Canonical inbound wire types for the example feature's `/api/v1` endpoints
//! (Go `internal/features/example/contract` parity). This module is the single
//! source of the HTTP JSON shapes; the published facade `crate::api::v1`
//! re-exports these types so other services can construct payloads without
//! importing server internals.
//!
//! Leaf rule: the contract must not depend on any crate-internal module
//! (`tests/architecture.rs`: contract-is-leaf), so the published facade drags
//! in nothing but serde/validator types.

pub mod create_item;
pub mod list_items;

pub use create_item::{CreateItemRequest, ItemResponse};
pub use list_items::{ListItemsRequest, ListItemsResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn create_request_validates_length() {
        let req = CreateItemRequest {
            name: "".to_string(),
        };
        assert!(req.validate().is_err());
        let req = CreateItemRequest {
            name: "a".repeat(256),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn list_request_validates_range() {
        let mut req = ListItemsRequest {
            limit: Some(200),
            offset: Some(-1),
        };
        assert!(req.validate().is_err());
        req.limit = Some(50);
        req.offset = Some(0);
        assert!(req.validate().is_ok());
    }
}
