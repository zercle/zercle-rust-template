//! HTTP server module
//!
//! This module contains the server setup and configuration.
//! It provides the Server struct for running the axum HTTP server.

use crate::config::Settings;
use crate::domain::usecases::{TaskUsecase, UserUsecase};
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::http::routes::create_router;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

/// Server configuration and state
pub struct Server {
    router: axum::Router,
    addr: SocketAddr,
}

impl Server {
    /// Create a new server instance
    ///
    /// # Arguments
    /// * `router` - The configured axum router
    /// * `addr` - Socket address to bind to
    ///
    /// # Returns
    /// A new Server instance
    pub fn new(router: axum::Router, addr: SocketAddr) -> Self {
        Self { router, addr }
    }

    /// Create a server from settings and dependencies
    ///
    /// # Arguments
    /// * `settings` - Application settings
    /// * `user_usecase` - User use case instance
    /// * `task_usecase` - Task use case instance
    /// * `database` - Database connection
    ///
    /// # Returns
    /// Configured Server instance
    pub fn from_dependencies(
        settings: &Settings,
        user_usecase: Arc<dyn UserUsecase>,
        task_usecase: Arc<dyn TaskUsecase>,
        database: &Database,
    ) -> Result<Self> {
        let addr = format!("{}:{}", settings.server.host, settings.server.port)
            .parse()
            .context("Failed to parse server address")?;

        let router = create_router(user_usecase, task_usecase, settings, database.pool().clone());

        Ok(Self::new(router, addr))
    }

    /// Get the server address
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Run the server with graceful shutdown
    ///
    /// This method:
    /// 1. Creates a TCP listener on the configured address
    /// 2. Starts the axum server with the router
    /// 3. Sets up signal handlers for graceful shutdown (SIGINT, SIGTERM)
    /// 4. Waits for shutdown signal before stopping
    ///
    /// # Returns
    /// Result indicating success or error
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .context("Failed to bind to address")?;

        tracing::info!("Server listening on {}", self.addr);

        // Create the server
        let server = axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal());

        // Run the server
        server.await.context("Server failed")?;

        Ok(())
    }
}

/// Create a future that completes when a shutdown signal is received
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received terminate signal, shutting down...");
        }
    }
}

/// Run the server with hot reload support for development
///
/// This is a convenience function for development that prints
/// startup information and handles errors gracefully.
pub async fn run_server(
    router: axum::Router,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new(router, addr);
    server.run().await
}
