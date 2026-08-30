//! STUB FEATURE — delete src/features/example to start your project.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Payload for `POST /items`.
///
/// `Serialize` is provided so external consumers (via the published facade)
/// can construct and serialize payloads symmetrically.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateItemRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
}

/// JSON representation of an `Item`.
///
/// Timestamps are RFC 3339 strings; mapping from the domain entity lives in
/// the application layer (`application::usecase`), keeping this module free of
/// any crate-internal dependency.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ItemResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}
