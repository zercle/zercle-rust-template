//! Cross-cutting platform concerns (Go `internal/platform/*` parity):
//! config, database/cache adapters, boundary errors, health, telemetry,
//! the HTTP/gRPC server shell, and middleware.
//!
//! Platform code is feature-agnostic by rule — it may never import
//! `crate::features::…` (enforced by `tests/architecture.rs`). Features
//! depend on platform, never the reverse.

pub mod config;
pub mod db;
pub mod errors;
pub mod health;
pub mod middleware;
pub mod server;
pub mod telemetry;
pub mod valkey;
