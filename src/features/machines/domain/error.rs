//! Machines domain sentinel errors (Go `domain/errors.go` parity, port-spec
//! §3/§5.2c).
//!
//! Go sentinels: `ErrMachineNotFound` = "machine not found", `ErrInvalidID` =
//! "machine id is invalid", `ErrInvalidMachineLabel` = "machine label is
//! invalid", `ErrUnsupportedCoin` = "unsupported coin".

/// Domain error. The four semantic sentinels map to boundary codes; `Internal`
/// forwards the cause to the boundary for a 500.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("machine not found")]
    MachineNotFound,
    #[error("machine id is invalid")]
    InvalidId,
    #[error("machine label is invalid")]
    InvalidMachineLabel,
    #[error("unsupported coin")]
    UnsupportedCoin,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::MachineNotFound, Error::MachineNotFound)
            | (Error::InvalidId, Error::InvalidId)
            | (Error::InvalidMachineLabel, Error::InvalidMachineLabel)
            | (Error::UnsupportedCoin, Error::UnsupportedCoin) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
