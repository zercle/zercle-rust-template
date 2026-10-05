//! reporting — distributed-vending-machines demo feature (Go
//! `internal/features/reporting` parity, port-spec §5.4).
//!
//! Full clean-architecture slice: a pure `domain` overview + leaderboard, a
//! `contract` wire surface, an outbound read-only `port` that aggregates the
//! catalog/machines/sales tables, an `application` use case, and both adapter
//! rings (axum HTTP + tonic gRPC driving; postgres driven) wired by `di`.
//! Reporting owns no schema, so it contributes no migrations.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      Overview + MachineSales + domain errors (innermost)
//! port/        outbound (driven) read-only Repository port
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres)
//! di.rs        composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Route (Go §5.4a): `GET /reports/summary`.

pub mod adapter;
pub mod application;
pub mod contract;
pub mod di;
pub mod domain;
pub mod port;

pub use di::{Wired, register};
