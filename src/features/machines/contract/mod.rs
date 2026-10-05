//! machines contract — canonical inbound wire types for `/api/v1` machines
//! (Go `internal/features/machines/contract` parity, port-spec §5.2b).
//!
//! Leaf rule: no crate-internal imports (`tests/architecture.rs`:
//! contract-is-leaf). `CoinBank` JSON is an object keyed by denomination in
//! cents as a string, e.g. `{"25":2,"100":1}` (serde_json stringifies integer
//! map keys).

pub mod create_machine;
pub mod list_machines;

pub use create_machine::{CreateMachineRequest, MachineResponse};
pub use list_machines::{ListMachinesRequest, ListMachinesResponse, RestockBankRequest};

/// Structural coin check shared by `initial_coins` (omitempty) and `coins`
/// (required, min=1): every supplied denomination must be positive. Go tag
/// parity: `dive,gt=0`. The *supported-denomination* rule is business logic
/// and lives in `domain::validate_coins`.
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
    fn create_request_requires_label_and_positive_initial_coins() {
        let ok = CreateMachineRequest {
            label: "lobby".to_string(),
            initial_coins: vec![25, 100],
        };
        assert!(ok.validate().is_ok());

        let empty_label = CreateMachineRequest {
            label: String::new(),
            initial_coins: vec![],
        };
        assert!(empty_label.validate().is_err());

        let bad_coin = CreateMachineRequest {
            label: "lobby".to_string(),
            initial_coins: vec![0],
        };
        assert!(bad_coin.validate().is_err());
    }

    #[test]
    fn restock_requires_non_empty_positive_coins() {
        let empty = RestockBankRequest { coins: vec![] };
        assert!(empty.validate().is_err());
        let negative = RestockBankRequest { coins: vec![-5] };
        assert!(negative.validate().is_err());
        let ok = RestockBankRequest { coins: vec![5] };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn machine_response_serializes_coin_bank_as_string_keys() {
        let mut coin_bank = std::collections::BTreeMap::new();
        coin_bank.insert(25, 2);
        coin_bank.insert(100, 1);
        let resp = MachineResponse {
            id: "id".to_string(),
            label: "lobby".to_string(),
            coin_bank,
            created_at: "t1".to_string(),
            updated_at: "t2".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(
            json.contains(r#""coin_bank":{"25":2,"100":1}"#),
            "got {json}"
        );
    }
}
