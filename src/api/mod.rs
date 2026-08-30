//! Published API surface of the service (Go `pkg/api` parity): typed inbound
//! contracts and error codes that other services may import to construct
//! payloads and interpret error envelopes without importing server internals.
//!
//! Internal code must not depend on this facade — the dependency is strictly
//! outward-only (enforced by `tests/architecture.rs`:
//! published-contract-is-outward-only).

pub mod v1;
