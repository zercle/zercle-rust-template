//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Domain sentinel errors for the example feature (Go `domain/errors.go`
//! parity). `Internal` carries infrastructure failures (sqlx, …) that don't
//! map to a semantic sentinel — the typed equivalent of Go's free-form
//! `fmt.Errorf("...: %w", err)` wrapping.
//!
//! Mapping to the shared boundary `AppError` is registered at the composition
//! edge in the feature's `di` module (Go `apperrors.RegisterSentinel` parity),
//! so this module stays dependency-free.

/// Domain error type. The three semantic sentinels map to boundary error
/// codes; `Internal` forwards the cause to the boundary for a 500.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("item not found")]
    NotFound,
    #[error("item name is invalid")]
    InvalidName,
    #[error("item id is invalid")]
    InvalidId,
    #[error("internal error")]
    Internal { cause: Option<anyhow::Error> },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::NotFound, Error::NotFound)
            | (Error::InvalidName, Error::InvalidName)
            | (Error::InvalidId, Error::InvalidId) => true,
            (Error::Internal { cause: a }, Error::Internal { cause: b }) => {
                a.as_ref().map(anyhow::Error::to_string) == b.as_ref().map(anyhow::Error::to_string)
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
