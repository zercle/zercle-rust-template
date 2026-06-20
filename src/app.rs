//! Composition root: builds `AppState` and runs the HTTP + gRPC servers (decision D2).
//!
//! Mirrors `internal/app/app.go` (structure.md §13). Build order:
//!
//! 1. Telemetry init (tracing + OTel + Prometheus).
//! 2. PostgreSQL pool + readiness checker.
//! 3. Valkey client + readiness checker.
//! 4. Example feature wiring (sqlx repository → service).
//! 5. [`AppState`] assembly.
//!
//! [`run`](crate::app::run) then delegates to [`shared::server::run`] which starts axum +
//! tonic and orchestrates the ordered graceful shutdown.

use std::sync::Arc;

use crate::{
    config::Config,
    features::example::{domain::Repository, repository::PgRepository, service::ServiceImpl},
    infrastructure::{PgChecker, ValkeyChecker, new_client as new_valkey_client, new_pool},
    shared::{
        health::Registry as HealthRegistry,
        telemetry::{Telemetry, init as init_telemetry},
    },
};

/// Process-wide application state. Cloned cheaply via [`Arc`] (the underlying
/// pools and registries are already `Arc`-based).
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: sqlx::PgPool,
    pub valkey: redis::aio::ConnectionManager,
    pub health: Arc<HealthRegistry>,
    pub example_service: Arc<ServiceImpl>,
}

/// Build metadata, populated at compile time via `option_env!` (see
/// `src/main.rs`). Re-exported from `app` so [`run`](crate::app::run) can log
/// them without depending on the binary.
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

/// Build the application state. Mirrors Go `app.Build`.
pub async fn build(cfg: Config) -> anyhow::Result<(AppState, Telemetry)> {
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

    let repo: Arc<dyn Repository> = Arc::new(PgRepository::new(db.clone()));
    let service = Arc::new(ServiceImpl::new(
        repo,
        cfg.example.default_page_size as i32,
        cfg.example.max_page_size as i32,
        cfg.example.max_name_length as i32,
    ));

    let state = AppState {
        cfg: Arc::new(cfg),
        db,
        valkey,
        health: Arc::new(health),
        example_service: service,
    };

    Ok((state, telemetry))
}

/// Run the application until SIGTERM/SIGINT, then perform an ordered graceful
/// shutdown. This is the top-level orchestrator used by the `server` binary
/// and by integration tests.
pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let (state, telemetry) = build(cfg).await?;
    crate::shared::server::run(state, telemetry).await
}
