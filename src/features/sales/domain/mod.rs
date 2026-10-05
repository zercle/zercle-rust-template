//! sales domain — innermost layer: coin arithmetic, the purchase rule, and
//! sentinel errors (Go `internal/features/sales/domain` parity, port-spec
//! §5.3c).
//!
//! `SupportedDenominations` is duplicated from machines' `Denominations` on
//! purpose — the architecture gates forbid importing another feature's domain.

pub mod coins;
pub mod error;
pub mod purchase;

pub use coins::{CoinBank, SUPPORTED_DENOMINATIONS, make_change, sum_coins, validate_coins};
pub use error::Error;
pub use purchase::{PurchaseRecord, SaleProduct, purchase};
