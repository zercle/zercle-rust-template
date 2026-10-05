//! Create-machine wire types (Go `contract/create_machine.go` parity, port-spec
//! §5.2b).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::validate_positive_coins;

/// Payload for `POST /machines` (Go `CreateMachineRequest`, verbatim tags:
/// `label:"required" initial_coins:"omitempty,dive,gt=0"`).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateMachineRequest {
    #[validate(length(min = 1))]
    pub label: String,
    #[serde(default)]
    #[validate(custom(function = "validate_positive_coins"))]
    pub initial_coins: Vec<i32>,
}

/// JSON representation of a vending machine (Go `MachineResponse`, verbatim
/// fields). `coin_bank` is an object keyed by denomination in cents as a
/// string (e.g. `{"25":2,"100":1}`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MachineResponse {
    pub id: String,
    pub label: String,
    pub coin_bank: BTreeMap<i32, i32>,
    pub created_at: String,
    pub updated_at: String,
}
