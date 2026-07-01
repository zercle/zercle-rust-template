//! Server binary entry point.
//!
//! Loads and validates config, then delegates to [`zercle_rust_template::run`]
//! which builds [`AppState`](crate::app::AppState), starts the HTTP and gRPC
//! servers, and orchestrates the ordered graceful shutdown.

use std::process::ExitCode;

use zercle_rust_template::config::Config;

/// Compile-time build metadata. Overridden by the build system via
/// `option_env!` so the binary runs without extra build flags.
const VERSION: &str = match option_env!("VERSION") {
    Some(v) => v,
    None => "dev",
};
const COMMIT_SHA: &str = match option_env!("COMMIT_SHA") {
    Some(v) => v,
    None => "unknown",
};
const BUILD_TIME: &str = match option_env!("BUILD_TIME") {
    Some(v) => v,
    None => "unknown",
};

#[tokio::main]
async fn main() -> ExitCode {
    eprintln!("server {VERSION} ({COMMIT_SHA}) built {BUILD_TIME} starting");

    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = validator::Validate::validate(&cfg).map_err(|e| anyhow::anyhow!(e)) {
        eprintln!("invalid config: {e:#}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = cfg.validate_cross() {
        eprintln!("invalid config: {e:#}");
        return ExitCode::FAILURE;
    }

    match zercle_rust_template::run_with_config(cfg).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("server stopped with error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
