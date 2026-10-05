//! Inbound use-case port + implementation (Go
//! `internal/features/catalog/usecase` parity, port-spec §5.1d). Driving
//! adapters (next wave) consume [`Service`]; [`Usecase`] orchestrates the
//! domain and the outbound port and owns domain ↔ contract mapping.

pub mod service;
pub mod usecase;

#[cfg(test)]
pub use service::MockService;
pub use service::{Service, SharedService};
pub use usecase::Usecase;
