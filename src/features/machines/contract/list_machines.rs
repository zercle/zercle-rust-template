//! List/restock wire types (Go `contract/list_machines.go` parity, port-spec
//! §5.2b).

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create_machine::MachineResponse;
use super::validate_positive_coins;

/// Query parameters for `GET /machines` (Go `ListMachinesRequest`, verbatim
/// tags: `limit:"min=0" offset:"min=0"`).
#[derive(Debug, Clone, Default, Deserialize, Validate)]
pub struct ListMachinesRequest {
    #[validate(range(min = 0))]
    pub limit: Option<i32>,
    #[validate(range(min = 0))]
    pub offset: Option<i32>,
}

/// Response body for `GET /machines` (Go `ListMachinesResponse`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListMachinesResponse {
    pub machines: Vec<MachineResponse>,
}

/// Payload for `POST /machines/{id}/bank` (Go `RestockBankRequest`, verbatim
/// tags: `coins:"required,min=1,dive,gt=0"`). The path `id` is not part of the
/// JSON body.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RestockBankRequest {
    #[validate(length(min = 1), custom(function = "validate_positive_coins"))]
    pub coins: Vec<i32>,
}
