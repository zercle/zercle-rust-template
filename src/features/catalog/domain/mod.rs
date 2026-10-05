//! catalog domain — innermost layer: the `Product` entity + sentinel errors
//! (Go `internal/features/catalog/domain` parity, port-spec §5.1c).
//!
//! Depends on nothing crate-internal (`tests/architecture.rs`:
//! domain-is-innermost) — only stdlib-adjacent crates (uuid, time, thiserror,
//! anyhow).

pub mod error;
pub mod product;

pub use error::Error;
pub use product::Product;
