//! Purchase wire types (Go `contract/purchase.go` parity, port-spec §5.3b).

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::validate_positive_coins;

/// Payload for `POST /purchases` (Go `PurchaseRequest`, verbatim tags:
/// `machine_id:"required,uuid" product_id:"required,uuid"
/// coins:"required,min=1,dive,gt=0"`).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PurchaseRequest {
    #[validate(length(min = 1))]
    pub machine_id: String,
    #[validate(length(min = 1))]
    pub product_id: String,
    #[validate(length(min = 1), custom(function = "validate_positive_coins"))]
    pub coins: Vec<i32>,
}

/// JSON representation of a completed purchase (Go `PurchaseResponse`,
/// verbatim fields).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PurchaseResponse {
    pub id: String,
    pub machine_id: String,
    pub product_id: String,
    pub price_cents: i32,
    pub total_inserted_cents: i32,
    pub change_cents: i32,
    pub change_coins: Vec<i32>,
    pub purchased_at: String,
}
