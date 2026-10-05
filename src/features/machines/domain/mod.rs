//! machines domain — innermost layer: the `Machine` entity, coin rules, and
//! sentinel errors (Go `internal/features/machines/domain` parity, port-spec
//! §5.2c).
//!
//! Depends on nothing crate-internal (`tests/architecture.rs`:
//! domain-is-innermost). `Denominations` is duplicated from sales'
//! `SupportedDenominations` on purpose — the architecture gates forbid
//! importing another feature's domain (Go doc note, §5.2c).

pub mod error;
pub mod machine;

pub use error::Error;
pub use machine::{CoinBank, DENOMINATIONS, Machine, add_coins, validate_coins};
