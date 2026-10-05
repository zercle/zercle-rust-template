//! Inbound use-case service + implementation (Go
//! `internal/features/catalog/usecase` parity, port-spec §5.1d). The `handler`
//! adapters consume [`Service`]; [`Usecase`] orchestrates the domain and the
//! outbound repository interface and owns domain ↔ contract mapping.
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
