//! Integration test module
//!
//! Integration tests verify the complete HTTP API layer with real database connections.
//! These tests require a running PostgreSQL database.

use std::sync::Arc;
use zercle_rust_template::internal::infrastructure::{config::Config, di::Container};

/// Set up a test container with test configuration
///
/// This function loads test configuration from environment variables
/// and creates a new DI container. The database must be running
/// for these tests to pass.
pub async fn setup_test_container() -> Arc<Container> {
    // Load configuration from environment or use defaults
    let config = Config::load().expect("Failed to load configuration");

    // Create container - this will also run migrations
    // Note: For tests, you should use a test database
    let container = Container::new(config)
        .await
        .expect("Failed to create test container");

    Arc::new(container)
}
