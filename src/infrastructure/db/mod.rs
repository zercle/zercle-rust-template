//! Database infrastructure module
//!
//! This module provides database connection management, migrations,
//! and related utilities for PostgreSQL using sqlx.

pub mod connection;
pub mod migrations;
pub mod postgres_repository;

// Re-export commonly used types and functions
pub use connection::{connect, health_check, Database, DbPool};
pub use migrations::{migrate_from_settings, Migrations};

// Re-export repository types
pub use postgres_repository::{PostgresTaskRepository, PostgresUserRepository, RepositoryError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that the module exports the expected types
        // This is a compile-time check
        let _ = || -> Option<Database> { None };
        let _ = || -> Option<DbPool> { None };
        let _ = || -> Option<Migrations> { None };
        let _ = || -> Option<PostgresUserRepository> { None };
        let _ = || -> Option<PostgresTaskRepository> { None };
    }
}
