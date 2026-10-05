//! Driven (outbound) adapters for catalog: `postgres.rs` (sqlx implementation
//! of `port::Repository`, Go `repository/postgres/repository.go`) and its
//! cache-aside decorator `cached.rs` (Go `cached_repository.go`, §5.1j). The
//! feature's migration SQL is owned here under `postgres/migrations/`.

pub mod cached;
pub mod postgres;
