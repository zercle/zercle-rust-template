//! machines — distributed-vending-machines demo feature (Go
//! `internal/features/machines` parity, port-spec §5.2).
//!
//! Full clean-architecture slice: pure `domain` rules (coin denominations +
//! bank arithmetic), a `contract` wire surface, an outbound `repository`
//! interface, a `usecase` service, and the `handler` (axum HTTP + tonic gRPC)
//! plus `repository/postgres` (sqlx) adapters wired by `di`.
//!
//! ```text
//! contract/            canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/              Machine entity + coin rules + domain errors (innermost)
//! repository/          outbound (driven) Repository interface
//! repository/postgres/ sqlx impl + migrations/
//! usecase/             inbound (driving) Service interface + Usecase implementation
//! handler/             axum HTTP + tonic gRPC handlers
//! di.rs                composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Routes (Go §5.2a): `POST /machines` · `GET /machines` ·
//! `GET /machines/{id}` · `POST /machines/{id}/bank`.

pub mod contract;
pub mod di;
pub mod domain;
pub mod handler;
pub mod repository;
pub mod usecase;

pub use di::{Wired, register};
