//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Outbound (driven) ports for the example feature (Go
//! `internal/features/example/port` parity). The application layer consumes
//! these traits; driven adapters under `adapter/out` implement them.

pub mod repository;

#[cfg(test)]
pub use repository::MockRepository;
pub use repository::Repository;
