//! catalog — distributed-vending-machines demo feature (Go
//! `internal/features/catalog` parity, port-spec §5.1).
//!
//! Full clean-architecture slice: pure `domain` rules, a `contract` wire
//! surface, an outbound `port`, an `application` use case, and both adapter
//! rings (axum HTTP + tonic gRPC driving; postgres + Valkey cache-aside
//! driven) wired by `di`.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      Product entity + domain errors (innermost)
//! port/        outbound (driven) Repository port
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres, cache-aside)
//! di.rs        composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Routes (Go §5.1a): `POST /products` · `GET /products` · `GET /products/{id}`.

pub mod adapter;
pub mod application;
pub mod contract;
pub mod di;
pub mod domain;
pub mod port;

pub use di::{Wired, register};
