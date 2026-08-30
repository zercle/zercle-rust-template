//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Inbound use-case port + implementation (Go
//! `internal/features/example/application/{service,usecase}.go` parity).
//! Driving adapters under `adapter/in` consume the [`Service`] port; the
//! implementation orchestrates the domain and the outbound ports and maps to
//! and from the wire `contract` types at the boundary.

pub mod service;
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
