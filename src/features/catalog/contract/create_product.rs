//! Create + read product wire types (Go `contract/create_product.go` parity,
//! port-spec §5.1b).

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Payload for `POST /products` (Go `CreateProductRequest`, verbatim tags:
/// `name:"required" price_cents:"required,gt=0" stock:"gte=0"`).
///
/// Only structural constraints live here; the deployment-configurable
/// name-length cap is enforced in the usecase (Go design note, §5.1b).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(range(min = 1))]
    pub price_cents: i32,
    #[validate(range(min = 0))]
    pub stock: i32,
}

/// JSON representation of a catalog product (Go `ProductResponse`, verbatim
/// fields). Timestamps are RFC 3339 strings; domain → contract mapping lives
/// in `application::usecase`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub price_cents: i32,
    pub stock: i32,
    pub created_at: String,
    pub updated_at: String,
}
