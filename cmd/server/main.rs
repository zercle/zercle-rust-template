use std::net::SocketAddr;
use std::sync::Arc;

use tokio::signal;
use tracing::{info, error};

use zercle_rust_template::internal::infrastructure::{
    config::Config,
    di::Container,
    http::create_router,
    logger::init_logging,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!(%panic_info, "Application panicked");
    }));

    // Load configuration
    let config = Config::load()?;

    // Initialize logging
    init_logging(&config.logging.level, &config.logging.format)?;

    info!("Starting server...");

    // Create DI container (initializes all dependencies)
    let container = Arc::new(Container::new(config).await?);

    // Create router
    let app = create_router(container.clone());

    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], container.config.app.port));
    info!("Server listening on {}", addr);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

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
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown...");
}
