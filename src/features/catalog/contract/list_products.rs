//! List-products wire types (Go `contract/list_products.go` parity, port-spec
//! §5.1b).

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create_product::ProductResponse;

/// Query parameters for `GET /products` (Go `ListProductsRequest`, verbatim
/// tags: `limit:"min=0" offset:"min=0"`). `None` means "not supplied"; the
/// usecase applies the configured default.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
pub struct ListProductsRequest {
    #[validate(range(min = 0))]
    pub limit: Option<i32>,
    #[validate(range(min = 0))]
    pub offset: Option<i32>,
}

/// Response body for `GET /products` (Go `ListProductsResponse`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListProductsResponse {
    pub products: Vec<ProductResponse>,
}
