//! Catalog domain sentinel errors (Go `domain/errors.go` parity, port-spec
//! §3/§5.1c).
//!
//! Go sentinels and their exact messages:
//! `ErrProductNotFound` = "product not found", `ErrInvalidID` = "product id is
//! invalid", `ErrInvalidProductName` = "product name is invalid",
//! `ErrInvalidPrice` = "product price is invalid".
//!
//! Mapping to the shared boundary `AppError` is registered at the composition
//! edge in the feature's `di` module (Go `apperrors.RegisterSentinel` parity),
//! so this module stays dependency-free. `Internal` carries infrastructure
//! failures (sqlx, …) that are not semantic sentinels.

/// Domain error. The four semantic sentinels map to boundary codes; `Internal`
/// forwards the cause to the boundary for a 500.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("product not found")]
    ProductNotFound,
    #[error("product id is invalid")]
    InvalidId,
    #[error("product name is invalid")]
    InvalidProductName,
    #[error("product price is invalid")]
    InvalidPrice,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::ProductNotFound, Error::ProductNotFound)
            | (Error::InvalidId, Error::InvalidId)
            | (Error::InvalidProductName, Error::InvalidProductName)
            | (Error::InvalidPrice, Error::InvalidPrice) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
