//! catalog contract — canonical inbound wire types for `/api/v1` products
//! (Go `internal/features/catalog/contract` parity, port-spec §5.1b).
//!
//! Leaf rule: no crate-internal imports (`tests/architecture.rs`:
//! contract-is-leaf), so the published `crate::api::v1` facade drags in
//! nothing but serde/validator types. Only structural constraints live here;
//! the name-length limit is deployment-configurable and enforced in the
//! usecase, so a hardcoded `max=` would drift (Go design note, §5.1b).

pub mod create_product;
pub mod list_products;

pub use create_product::{CreateProductRequest, ProductResponse};
pub use list_products::{ListProductsRequest, ListProductsResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn create_request_requires_name_and_positive_price() {
        let ok = CreateProductRequest {
            name: "cola".to_string(),
            price_cents: 150,
            stock: 3,
        };
        assert!(ok.validate().is_ok());

        let empty_name = CreateProductRequest {
            name: String::new(),
            price_cents: 150,
            stock: 0,
        };
        assert!(empty_name.validate().is_err());

        let zero_price = CreateProductRequest {
            name: "cola".to_string(),
            price_cents: 0,
            stock: 0,
        };
        assert!(zero_price.validate().is_err());

        let negative_stock = CreateProductRequest {
            name: "cola".to_string(),
            price_cents: 150,
            stock: -1,
        };
        assert!(negative_stock.validate().is_err());
    }

    #[test]
    fn list_request_rejects_negative_pagination() {
        let mut req = ListProductsRequest {
            limit: Some(-1),
            offset: Some(0),
        };
        assert!(req.validate().is_err());
        req.limit = Some(0);
        req.offset = Some(-1);
        assert!(req.validate().is_err());
        req.offset = Some(0);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn product_response_round_trips_json() {
        let resp = ProductResponse {
            id: "id".to_string(),
            name: "n".to_string(),
            price_cents: 10,
            stock: 1,
            created_at: "t1".to_string(),
            updated_at: "t2".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            json,
            r#"{"id":"id","name":"n","price_cents":10,"stock":1,"created_at":"t1","updated_at":"t2"}"#
        );
    }
}
