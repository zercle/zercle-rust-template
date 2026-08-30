//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Innermost layer: entities + domain errors. The domain depends on nothing
//! crate-internal (`tests/architecture.rs`: domain-is-innermost) — it imports
//! only stdlib-adjacent crates (uuid, time, thiserror).

pub mod error;
pub mod item;

pub use error::Error;
pub use item::Item;
