//! Reporting domain sentinel errors (Go `domain/errors.go` parity, port-spec
//! §3/§5.4c).
//!
//! Go sentinel: `ErrInvalidTopMachines` = "top machines limit is invalid".

/// Domain error. `InvalidTopMachines` maps to `INVALID_INPUT`; `Internal`
/// forwards the cause to the boundary for a 500.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("top machines limit is invalid")]
    InvalidTopMachines,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::InvalidTopMachines, Error::InvalidTopMachines) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
