use std::net::SocketAddr;
use axum::{routing::get, Router};
use zercle_rust_template::internal::infrastructure::config::Config;
use zercle_rust_template::internal::infrastructure::db::connection::Database;
use zercle_rust_template::internal::infrastructure::db::migrations::Migrations;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!(%panic_info, "Application panicked");
    }));

    fmt()
        .with_max_level(tracing::Level::INFO)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let config = Config::load()?;

    tracing::info!("Starting Zercle Rust Template application");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Environment: {}", config.app.env);

    let pool = Database::connect(&config).await?;
    tracing::info!("Database connected successfully");

    Migrations::run(&pool).await?;
    tracing::info!("Database migrations completed");

    let app = create_app();
    let addr = format!("{}:{}", config.app.host, config.app.port);
    let addr: SocketAddr = addr.parse()?;
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

fn create_app() -> Router {
    Router::new()
        .route("/health", get(health))
}
