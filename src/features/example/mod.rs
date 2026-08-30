//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Example feature sliced into clean-architecture layers (Go
//! `internal/features/example` parity):
//!
//! ```text
//! contract/    canonical inbound wire types (leaf; published via crate::api::v1)
//! domain/      entities + domain errors (innermost)
//! port/        outbound (driven) ports
//! application/ inbound use-case port + implementation
//! adapter/in/  driving adapters (axum HTTP, tonic gRPC)
//! adapter/out/ driven adapters (postgres)
//! di.rs        composition: wiring + sentinel → boundary error registration
//! ```
//!
//! To start a real project: `rm -rf src/features/example`, remove `pub mod
//! example;` from `src/features/mod.rs`, and drop the `example::di::register`
//! call (plus the `example` config section) — the platform and app shell do
//! not reference the feature from anywhere else.

pub mod adapter;
pub mod application;
pub mod contract;
pub mod di;
pub mod domain;
pub mod port;

pub use di::{Wired, register};
