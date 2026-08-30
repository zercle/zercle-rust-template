//! Composition root (Go `internal/app/app.go` parity). Wires the platform and
//! every feature in dependency order:
//!
//! 1. Telemetry init (tracing + OTel + Prometheus).
//! 2. PostgreSQL pool + readiness checker.
//! 3. Valkey client + readiness checker.
//! 4. [`AppState`] assembly.
//! 5. Feature wiring — the only feature symbol referenced anywhere outside the
//!    feature is `features::example::di`; adding a feature means adding one
//!    `di::register` call here.
//!
//! [`run`](app::run) then delegates to [`platform::server::run`] which starts
//! axum + tonic and orchestrates the ordered graceful shutdown.

use std::sync::Arc;

use axum::Router;

use crate::features::example::di as example_di;
use crate::platform::{
    config::Config,
    db::{PgChecker, new_pool},
    health::Registry as HealthRegistry,
    server::{self, AppState, GrpcRouter},
    telemetry::{Telemetry, init as init_telemetry},
    valkey::{ValkeyChecker, new_client as new_valkey_client},
};

/// Build metadata, populated at compile time via `option_env!` (see
/// `src/main.rs`). Re-exported here so [`build`](app::build) can log them
/// without depending on the binary.
pub const VERSION: &str = match option_env!("VERSION") {
    Some(v) => v,
    None => "dev",
};
pub const COMMIT_SHA: &str = match option_env!("COMMIT_SHA") {
    Some(v) => v,
    None => "unknown",
};
pub const BUILD_TIME: &str = match option_env!("BUILD_TIME") {
    Some(v) => v,
    None => "unknown",
};

/// Fully assembled application: server state plus the pre-built feature
/// routers (Go `server.Application` parity).
pub struct Built {
    pub state: AppState,
    pub telemetry: Telemetry,
    /// Raw feature HTTP router(s), pre-nested under their versioned prefixes.
    /// The server shell wraps them with shared routes + middleware exactly
    /// once, inside [`platform::server::run`].
    pub api: Router,
    /// tonic router with every feature's gRPC services.
    pub grpc: GrpcRouter,
}

/// Build the application. Mirrors Go `app.Build`.
pub async fn build(cfg: Config) -> anyhow::Result<Built> {
    let telemetry = init_telemetry(&cfg).map_err(|e| anyhow::anyhow!("init telemetry: {e}"))?;

    tracing::info!(
        version = VERSION,
        commit = COMMIT_SHA,
        build_time = BUILD_TIME,
        env = %cfg.app.environment,
        "starting server"
    );

    let db = new_pool(&cfg)
        .await
        .map_err(|e| anyhow::anyhow!("connect postgres: {e:#}"))?;

    let valkey = new_valkey_client(&cfg)
        .await
        .map_err(|e| anyhow::anyhow!("connect valkey: {e:#}"))?;

    let mut health = HealthRegistry::new();
    health.add_readiness(Arc::new(PgChecker::new(db.clone())));
    health.add_readiness(Arc::new(ValkeyChecker::new(valkey.clone())));

    let state = AppState {
        cfg: Arc::new(cfg),
        db,
        valkey,
        health: Arc::new(health),
    };

    let wired = example_di::register(&state.cfg, state.db.clone());

    Ok(Built {
        state,
        telemetry,
        api: wired.http,
        grpc: wired.grpc,
    })
}

/// Run the application until SIGTERM/SIGINT, then perform an ordered graceful
/// shutdown. This is the top-level orchestrator used by the `server` binary
/// and by integration tests.
pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let built = build(cfg).await?;
    server::run(built.state, built.telemetry, built.api, built.grpc).await
}
