//! Catalog product entity (Go `domain/product.go` parity, port-spec §5.1c).

use time::OffsetDateTime;
use uuid::Uuid;

/// A sellable product.
///
/// Field-for-field parity with Go `domain.Product`; richer behavior (name/price
/// validation, timestamps) lives in `application::usecase`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub stock: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
