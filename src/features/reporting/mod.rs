//! reporting — distributed-vending-machines demo feature (Go
//! `internal/features/reporting` parity, port-spec §5.4).
//!
//! Full clean-architecture slice: a pure `domain` overview + leaderboard, a
//! `contract` wire surface, an outbound read-only `repository` interface that
//! aggregates the catalog/machines/sales tables, a `usecase` service, and the
//! `handler` (axum HTTP + tonic gRPC) plus `repository/postgres` (sqlx)
//! adapters wired by `di`. Reporting owns no schema, so it contributes no
//! migrations.
//!
//! ```text
//! contract/            canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/              Overview + MachineSales + domain errors (innermost)
//! repository/          outbound (driven) read-only Repository interface
//! repository/postgres/ sqlx read-only aggregation impl (no migrations/)
//! usecase/             inbound (driving) Service interface + Usecase implementation
//! handler/             axum HTTP + tonic gRPC handlers
//! di.rs                composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Route (Go §5.4a): `GET /reports/summary`.

pub mod contract;
pub mod di;
pub mod domain;
pub mod handler;
pub mod repository;
pub mod usecase;

pub use di::{Wired, register};
