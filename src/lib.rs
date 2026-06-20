//! zercle-rust-template — opinionated Rust (axum) microservice template.
//!
//! Composition root = `Arc<AppState>` (decision D2). See `app.rs` for the build order.

pub mod app;
pub mod config;
pub mod features;
pub mod infrastructure;
pub mod middleware;
pub mod shared;

pub use app::{AppState, build};
pub use config::Config;
pub use shared::server;
pub use shared::telemetry::Telemetry;

/// Top-level run entry point. Loads config from the environment, validates it,
/// then boots the full HTTP + gRPC server stack and waits for a shutdown signal.
///
/// Tests that need to run the application against real infrastructure call this
/// directly; the binary `server` (`src/main.rs`) is a thin wrapper.
pub async fn run() -> anyhow::Result<()> {
    let cfg = config::Config::load()?;
    validator::Validate::validate(&cfg).map_err(|e| anyhow::anyhow!(e))?;
    cfg.validate_cross()?;
    app::run(cfg).await
}

/// Convenience wrapper matching the spec contract: validate + run with an
/// already-loaded config. The binary uses this; tests typically use [`run`].
pub async fn run_with_config(cfg: Config) -> anyhow::Result<()> {
    validator::Validate::validate(&cfg).map_err(|e| anyhow::anyhow!(e))?;
    cfg.validate_cross()?;
    app::run(cfg).await
}
