//! Sales domain sentinel errors (Go `domain/errors.go` parity, port-spec
//! §3/§5.3c).
//!
//! Go sentinels: `ErrProductNotFound` = "product not found",
//! `ErrMachineNotFound` = "machine not found", `ErrInvalidID` = "sale id is
//! invalid", `ErrUnsupportedCoin` = "unsupported coin",
//! `ErrInsufficientPayment` = "insufficient payment", `ErrOutOfStock` =
//! "product out of stock", `ErrExactChangeRequired` = "exact change required".
//!
//! catalog's `ErrProductNotFound` and sales' are distinct errors with the same
//! text; each feature registers its own mapping at its `di` edge.

/// Domain error. The seven semantic sentinels map to boundary codes; `Internal`
/// forwards the cause to the boundary for a 500.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("product not found")]
    ProductNotFound,
    #[error("machine not found")]
    MachineNotFound,
    #[error("sale id is invalid")]
    InvalidId,
    #[error("unsupported coin")]
    UnsupportedCoin,
    #[error("insufficient payment")]
    InsufficientPayment,
    #[error("product out of stock")]
    OutOfStock,
    #[error("exact change required")]
    ExactChangeRequired,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::ProductNotFound, Error::ProductNotFound)
            | (Error::MachineNotFound, Error::MachineNotFound)
            | (Error::InvalidId, Error::InvalidId)
            | (Error::UnsupportedCoin, Error::UnsupportedCoin)
            | (Error::InsufficientPayment, Error::InsufficientPayment)
            | (Error::OutOfStock, Error::OutOfStock)
            | (Error::ExactChangeRequired, Error::ExactChangeRequired) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
