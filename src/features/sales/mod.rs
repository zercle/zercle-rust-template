//! STUB FEATURE (IO edge) — sales: distributed-vending-machines demo.
//!
//! Go `internal/features/sales` parity (port-spec §5.3). The pure-logic layers
//! are fully implemented and unit-tested; `adapter/` holds empty module
//! skeletons and `di.rs` is a warn-only stub until the next wave wires
//! postgres and axum/tonic.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      coins + purchase rules + domain errors (innermost)
//! port/        outbound (driven) Repository port (cross-feature reads)
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres) skeletons — next wave
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
