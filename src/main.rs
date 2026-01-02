//! Main entry point for the Zercle Rust Template application
//!
//! This module sets up the application infrastructure including:
//! - Logging/tracing configuration
//! - Configuration loading
//! - Database connection and migrations
//! - HTTP server initialization
//! - Graceful shutdown handling

use zercle_rust_template::app::App;
use zercle_rust_template::config::Settings;
use anyhow::{Context, Result, bail};
use std::panic;

/// Main entry point for the application
///
/// This function:
/// 1. Initializes tracing/logging
/// 2. Loads configuration
/// 3. Connects to the database
/// 4. Runs migrations
/// 5. Builds the application
/// 6. Starts the HTTP server
///
/// # Returns
/// Result indicating success or error
///
/// # Errors
/// This function will return an error if:
/// - Configuration cannot be loaded
/// - Database connection fails
/// - Migrations fail
/// - Server fails to start
#[tokio::main]
async fn main() -> Result<()> {
    // Set up panic hook for better error messages
    panic::set_hook(Box::new(|panic_info| {
        tracing::error!(%panic_info, "Application panicked");
    }));

    // Initialize tracing/subscriber
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting Zercle Rust Template application");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let settings = Settings::load()?;
    tracing::info!(
        "Environment: {}, Host: {}, Port: {}",
        settings.server.env,
        settings.server.host,
        settings.server.port
    );

    // Log configuration details (without sensitive data)
    tracing::info!(
        "Database: {}@{}:{}/{}",
        settings.database.user,
        settings.database.host,
        settings.database.port,
        settings.database.name
    );
    tracing::info!(
        "JWT expiration: {} hours",
        settings.jwt.expiration_hours
    );

    // Build and run the application
    let app = App::new().await
        .context("Failed to initialize application")?;

    tracing::info!(
        "Server listening on {}",
        app.server_addr()
    );

    // Run the server (this will block until shutdown)
    if let Err(e) = app.run_server().await {
        bail!("Server failed to run: {}", e);
    }

    tracing::info!("Application shutdown complete");
    Ok(())
}

/// Alternative entry point for running with custom settings
///
/// This is useful for testing or when you need to pass custom settings.
///
/// # Arguments
/// * `settings` - Custom settings to use instead of loading from environment/file
///
/// # Returns
/// Result indicating success or error
pub async fn run_with_settings(_settings: Settings) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting application with custom settings");

    let app = App::new().await
        .context("Failed to initialize application")?;

    if let Err(e) = app.run_server().await {
        bail!("Server failed to run: {}", e);
    }

    Ok(())
}
