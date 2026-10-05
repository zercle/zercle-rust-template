//! Outbound (driven) ports for the catalog feature (Go
//! `internal/features/catalog/repository` parity, port-spec §5.1e).

pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
