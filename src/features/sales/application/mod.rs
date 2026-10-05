//! Inbound use-case port + implementation (Go
//! `internal/features/sales/usecase` parity, port-spec §5.3d).

pub mod service;
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
