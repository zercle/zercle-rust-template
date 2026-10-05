//! Inbound use-case port + implementation (Go
//! `internal/features/reporting/usecase` parity, port-spec §5.4d).

pub mod service;
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
