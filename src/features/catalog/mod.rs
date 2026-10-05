//! catalog — distributed-vending-machines demo feature (Go
//! `internal/features/catalog` parity, port-spec §5.1).
//!
//! Full clean-architecture slice: pure `domain` rules, a `contract` wire
//! surface, an outbound `repository` interface, a `usecase` service, and the
//! `handler` (axum HTTP + tonic gRPC) plus `repository/postgres` (sqlx +
//! Valkey cache-aside) adapters wired by `di`.
//!
//! ```text
//! contract/            canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/              Product entity + domain errors (innermost)
//! repository/          outbound (driven) Repository interface
//! repository/postgres/ sqlx impl + Valkey cache-aside decorator + migrations/
//! usecase/             inbound (driving) Service interface + Usecase implementation
//! handler/             axum HTTP + tonic gRPC handlers
//! di.rs                composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Routes (Go §5.1a): `POST /products` · `GET /products` · `GET /products/{id}`.

pub mod contract;
pub mod di;
pub mod domain;
pub mod handler;
pub mod repository;
pub mod usecase;

pub use di::{Wired, register};
