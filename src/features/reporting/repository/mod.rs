//! Outbound read-only repository interface for reporting (Go
//! `internal/features/reporting/repository` parity, port-spec §5.4e). The
//! `usecase` layer depends on this abstraction; `repository/postgres`
//! implements it. May reference only this feature's domain.

pub mod postgres;
// The interface file is named `repository.rs` to match the Go/Bun templates,
// so this module shares its parent's name (clippy::module_inception).
#[allow(clippy::module_inception)]
pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
