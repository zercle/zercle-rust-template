//! STUB FEATURE — delete src/features/example to start your project.

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create_item::ItemResponse;

/// Query parameters for `GET /items`.
///
/// `None` fields mean "not supplied"; the application layer applies safe
/// defaults so a zero-value request never produces `LIMIT 0`.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
pub struct ListItemsRequest {
    #[validate(range(min = 0, max = 100))]
    pub limit: Option<i32>,
    #[validate(range(min = 0))]
    pub offset: Option<i32>,
}

/// Response body for `GET /items`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListItemsResponse {
    pub items: Vec<ItemResponse>,
}
