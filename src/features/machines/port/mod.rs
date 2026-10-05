//! Outbound (driven) ports for the machines feature (Go
//! `internal/features/machines/repository` parity, port-spec §5.2e).

pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
