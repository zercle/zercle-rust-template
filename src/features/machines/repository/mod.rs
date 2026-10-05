//! Outbound repository interface for the machines feature (Go
//! `internal/features/machines/repository` parity, port-spec §5.2e). The
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
