//! Outbound (driven) read-only ports for reporting (Go
//! `internal/features/reporting/repository` parity, port-spec §5.4e).

pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
