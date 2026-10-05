//! Outbound (driven) ports for the sales feature (Go
//! `internal/features/sales/repository` parity, port-spec §5.3e).

pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
