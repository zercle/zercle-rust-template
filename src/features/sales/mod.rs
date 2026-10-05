//! sales — distributed-vending-machines demo feature (Go
//! `internal/features/sales` parity, port-spec §5.3).
//!
//! Full clean-architecture slice: pure `domain` rules (coin validation,
//! purchase, change making), a `contract` wire surface, an outbound `port`
//! that reads catalog/machines tables through its own row projections, an
//! `application` use case, and both adapter rings (axum HTTP + tonic gRPC
//! driving; postgres driven) wired by `di`.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      coins + purchase rules + domain errors (innermost)
//! port/        outbound (driven) Repository port (cross-feature reads)
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres)
//! di.rs        composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Route (Go §5.3a): `POST /purchases`.

pub mod adapter;
pub mod application;
pub mod contract;
pub mod di;
pub mod domain;
pub mod port;

pub use di::{Wired, register};
