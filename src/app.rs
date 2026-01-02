//! Application builder module
//!
//! This module contains the main App struct that orchestrates the application
//! initialization, including configuration, database, repositories, use cases,
//! and server setup.

use crate::config::Settings;
use crate::domain::repositories::{TaskRepository, UserRepository};
use crate::domain::usecases::{TaskUsecaseImpl, UserUsecaseImpl};
use crate::infrastructure::db::connection::{connect, Database};
use crate::infrastructure::db::migrations::Migrations;
use crate::infrastructure::db::postgres_repository::{
    PostgresTaskRepository, PostgresUserRepository,
};
use crate::infrastructure::http::server::Server;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;

/// Main application struct that holds all dependencies and configuration
///
/// # Example
/// ```no_run
/// use zercle_rust_template::app::App;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let app = App::new().await?;
///     app.run_server().await
/// }
/// ```
pub struct App {
    /// Application settings
    settings: Settings,
    /// Database connection pool
    db: Database,
    /// User repository instance
    user_repo: Arc<dyn UserRepository>,
    /// Task repository instance
    task_repo: Arc<dyn TaskRepository>,
    /// User use case instance
    user_usecase: Arc<UserUsecaseImpl>,
    /// Task use case instance
    task_usecase: Arc<TaskUsecaseImpl>,
    /// Server instance
    server: Server,
}

impl App {
    /// Create a new application instance
    ///
    /// This method:
    /// 1. Loads configuration from environment/file
    /// 2. Connects to the database
    /// 3. Runs database migrations
    /// 4. Initializes repositories
    /// 5. Initializes use cases
    /// 6. Creates the HTTP server
    ///
    /// # Returns
    /// Result containing the App instance or an error
    ///
    /// # Errors
    /// This function can fail if:
    /// - Configuration cannot be loaded
    /// - Database connection fails
    /// - Migrations fail
    pub async fn new() -> Result<Self> {
        tracing::info!("Initializing application...");

        // Load configuration
        let settings = Settings::load()?;
        tracing::info!("Configuration loaded, environment: {}", settings.server.env);

        // Connect to database
        let db = Database::connect(&settings)
            .await
            .context("Failed to connect to database")?;
        tracing::info!("Database connected successfully");

        // Run migrations
        Migrations::run(db.pool())
            .await
            .context("Failed to run database migrations")?;
        tracing::info!("Database migrations completed");

        // Initialize repositories
        let user_repo: Arc<dyn UserRepository> =
            Arc::new(PostgresUserRepository::new(db.pool().clone()));
        let task_repo: Arc<dyn TaskRepository> =
            Arc::new(PostgresTaskRepository::new(db.pool().clone()));
        tracing::info!("Repositories initialized");

        // Initialize use cases
        let user_usecase = Arc::new(UserUsecaseImpl::new(user_repo.clone(), &settings));
        let task_usecase = Arc::new(TaskUsecaseImpl::new(task_repo.clone()));
        tracing::info!("Use cases initialized");

        // Create server
        let server =
            Server::from_dependencies(&settings, user_usecase.clone(), task_usecase.clone(), &db)
                .context("Failed to create server")?;
        tracing::info!("Server created successfully");

        Ok(Self {
            settings,
            db,
            user_repo,
            task_repo,
            user_usecase,
            task_usecase,
            server,
        })
    }

    /// Get a reference to the user use case
    ///
    /// # Returns
    /// Reference to the user use case implementation
    pub fn user_usecase(&self) -> &UserUsecaseImpl {
        &self.user_usecase
    }

    /// Get a reference to the task use case
    ///
    /// # Returns
    /// Reference to the task use case implementation
    pub fn task_usecase(&self) -> &TaskUsecaseImpl {
        &self.task_usecase
    }

    /// Get a reference to the database connection
    ///
    /// # Returns
    /// Reference to the database connection pool
    pub fn db_pool(&self) -> &Database {
        &self.db
    }

    /// Get a reference to the application settings
    ///
    /// # Returns
    /// Reference to the application settings
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Get the server address
    ///
    /// # Returns
    /// The socket address the server is configured to listen on
    pub fn server_addr(&self) -> &SocketAddr {
        self.server.addr()
    }

    /// Run the application server
    ///
    /// This method starts the HTTP server and handles graceful shutdown
    /// when receiving SIGINT or SIGTERM signals.
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// This function can fail if the server fails to start or run
    pub async fn run_server(self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting server on {}", self.server.addr());

        // Run the server
        self.server.run().await?;

        Ok(())
    }

    /// Get the user repository for testing purposes
    pub fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.user_repo.clone()
    }

    /// Get the task repository for testing purposes
    pub fn task_repository(&self) -> Arc<dyn TaskRepository> {
        self.task_repo.clone()
    }
}

/// Create a database connection pool from settings
///
/// This is a convenience function for testing and standalone usage.
///
/// # Arguments
/// * `settings` - Application settings
///
/// # Returns
/// Result containing the connection pool or an error
pub async fn create_pool(settings: &Settings) -> Result<sqlx::Pool<sqlx::Postgres>> {
    connect(settings).await
}
