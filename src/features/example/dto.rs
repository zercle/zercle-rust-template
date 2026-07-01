//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Transport DTOs for the example feature (axum HTTP + tonic gRPC bodies).
//! Mirrors Go `internal/features/example/dto/{create_item,list_items}.go`.
//!
//! Timestamps are formatted as RFC 3339 at the DTO boundary.

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use validator::Validate;

use crate::features::example::domain::Item;

/// Payload for `POST /items`.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateItemRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
}

/// JSON representation of an `Item`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ItemResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ItemResponse {
    pub fn from_item(item: &Item) -> Self {
        Self {
            id: item.id.to_string(),
            name: item.name.clone(),
            created_at: format_rfc3339(item.created_at),
            updated_at: format_rfc3339(item.updated_at),
        }
    }
}

impl From<&Item> for ItemResponse {
    fn from(item: &Item) -> Self {
        Self::from_item(item)
    }
}

/// Query / body parameters for `GET /items`.
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

impl From<Vec<Item>> for ListItemsResponse {
    fn from(items: Vec<Item>) -> Self {
        Self {
            items: items.iter().map(ItemResponse::from_item).collect(),
        }
    }
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item() -> Item {
        Item {
            id: uuid::Uuid::nil(),
            name: "alpha".to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn item_response_uses_rfc3339() {
        let r = ItemResponse::from_item(&sample_item());
        assert_eq!(r.id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(r.name, "alpha");
        assert_eq!(r.created_at, "1970-01-01T00:00:00Z");
        assert_eq!(r.updated_at, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn list_response_from_vec() {
        let v: ListItemsResponse = vec![sample_item(), sample_item()].into();
        assert_eq!(v.items.len(), 2);
    }

    #[test]
    fn create_request_validates_length() {
        let req = CreateItemRequest {
            name: "".to_string(),
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
