//! machines — distributed-vending-machines demo feature (Go
//! `internal/features/machines` parity, port-spec §5.2).
//!
//! Full clean-architecture slice: pure `domain` rules (coin denominations +
//! bank arithmetic), a `contract` wire surface, an outbound `port`, an
//! `application` use case, and both adapter rings (axum HTTP + tonic gRPC
//! driving; postgres driven) wired by `di`.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      Machine entity + coin rules + domain errors (innermost)
//! port/        outbound (driven) Repository port
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres)
//! di.rs        composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Routes (Go §5.2a): `POST /machines` · `GET /machines` ·
//! `GET /machines/{id}` · `POST /machines/{id}/bank`.

pub mod adapter;
pub mod application;
pub mod contract;
pub mod di;
pub mod domain;
pub mod port;

pub use di::{Wired, register};
