//! Infrastructure adapters: PostgreSQL pool + Valkey client + health checkers.

pub mod db;
pub mod valkey;

pub use db::{PgChecker, new_pool};
pub use valkey::{ValkeyChecker, new_client};
