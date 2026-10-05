//! Inbound use-case service + implementation (Go
//! `internal/features/machines/usecase` parity, port-spec §5.2d).
//!
//! May reference only this feature's domain, repository, contract, and usecase
//! modules.

pub mod service;
// The implementation file is named `usecase.rs` to match the Go/Bun templates,
// so this module shares its parent's name (clippy::module_inception).
#[allow(clippy::module_inception)]
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
