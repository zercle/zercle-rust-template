//! sales contract — canonical inbound wire type for `/api/v1/purchases` (Go
//! `internal/features/sales/contract` parity, port-spec §5.3b).
//!
//! Leaf rule: no crate-internal imports (`tests/architecture.rs`:
//! contract-is-leaf). The `machine_id`/`product_id` `uuid` tags from Go are
//! enforced by the usecase's single id-parsing path (Go also parses there), so
//! the contract carries only the structural `required` check.

pub mod purchase;

pub use purchase::{PurchaseRequest, PurchaseResponse};

/// Structural coin check (Go tag parity: `dive,gt=0`): every supplied coin
/// must be positive. The supported-denomination rule is business logic in
/// `domain::validate_coins`.
pub(crate) fn validate_positive_coins(coins: &[i32]) -> Result<(), validator::ValidationError> {
    if coins.iter().all(|c| *c > 0) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("gt"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn purchase_requires_ids_and_positive_coins() {
        let ok = PurchaseRequest {
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            coins: vec![25],
        };
        assert!(ok.validate().is_ok());

        let missing_id = PurchaseRequest {
            machine_id: String::new(),
            product_id: "p".to_string(),
            coins: vec![25],
        };
        assert!(missing_id.validate().is_err());

        let no_coins = PurchaseRequest {
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            coins: vec![],
        };
        assert!(no_coins.validate().is_err());

        let bad_coin = PurchaseRequest {
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            coins: vec![0],
        };
        assert!(bad_coin.validate().is_err());
    }

    #[test]
    fn purchase_response_round_trips_json() {
        let resp = PurchaseResponse {
            id: "id".to_string(),
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            price_cents: 100,
            total_inserted_cents: 125,
            change_cents: 25,
            change_coins: vec![25],
            purchased_at: "t".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""change_coins":[25]"#), "got {json}");
    }
}
