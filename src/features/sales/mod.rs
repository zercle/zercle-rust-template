//! sales — distributed-vending-machines demo feature (Go
//! `internal/features/sales` parity, port-spec §5.3).
//!
//! Full clean-architecture slice: pure `domain` rules (coin validation,
//! purchase, change making), a `contract` wire surface, an outbound
//! `repository` interface that reads catalog/machines tables through its own
//! row projections, a `usecase` service, and the `handler` (axum HTTP + tonic
//! gRPC) plus `repository/postgres` (sqlx) adapters wired by `di`.
//!
//! ```text
//! contract/            canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/              coins + purchase rules + domain errors (innermost)
//! repository/          outbound (driven) Repository interface (cross-feature reads)
//! repository/postgres/ sqlx impl + migrations/
//! usecase/             inbound (driving) Service interface + Usecase implementation
//! handler/             axum HTTP + tonic gRPC handlers
//! di.rs                composition: sentinel → boundary error mapping + registry entry
//! ```
//!
//! Route (Go §5.3a): `POST /purchases`.

pub mod contract;
pub mod di;
pub mod domain;
pub mod handler;
pub mod repository;
pub mod usecase;

pub use di::{Wired, register};
