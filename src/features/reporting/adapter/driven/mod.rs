//! Driven (outbound) adapters for reporting. `postgres.rs` — the read-only
//! aggregation adapter — lands in the next wave. Reporting owns no schema, so
//! there is no `migrations/` directory here.

pub mod postgres;
