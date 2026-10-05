//! Inbound use-case port + implementation (Go
//! `internal/features/machines/usecase` parity, port-spec §5.2d).

pub mod service;
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
