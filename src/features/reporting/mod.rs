//! STUB FEATURE (IO edge) — reporting: distributed-vending-machines demo.
//!
//! Go `internal/features/reporting` parity (port-spec §5.4). The pure-logic
//! layers are fully implemented and unit-tested; `adapter/` holds empty module
//! skeletons and `di.rs` is a warn-only stub until the next wave wires
//! postgres and axum/tonic. Reporting owns no schema (it reads the other
//! features' tables), so it contributes no migrations.
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      Overview + MachineSales + domain errors (innermost)
//! port/        outbound (driven) read-only Repository port
//! application/ inbound Service port + Usecase implementation
//! adapter/     driving (http, grpc) + driven (postgres) skeletons — next wave
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
