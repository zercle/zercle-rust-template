//! reporting domain — innermost layer: read-only aggregates + sentinel errors
//! (Go `internal/features/reporting/domain` parity, port-spec §5.4c).
//!
//! Zero value is the empty-database report (an absent table contributes zero,
//! not an error). Depends on nothing crate-internal.

pub mod error;
pub mod summary;

pub use error::Error;
pub use summary::{MachineSales, Overview};
