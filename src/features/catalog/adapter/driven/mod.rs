//! Driven (outbound) adapters for catalog. `postgres.rs` — the sqlx
//! implementation of `port::Repository` — lands in the next wave. The
//! feature's migration SQL is owned here under `postgres/migrations/`.

pub mod postgres;
