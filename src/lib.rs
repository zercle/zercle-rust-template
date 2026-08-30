//! zercle-rust-template — opinionated Rust (axum) microservice template.
//!
//! Clean architecture (DDD) layout mirroring the Go template
//! (`internal/features/<name>/{contract,domain,port,application,adapter,di}`):
//!
//! * [`platform`] — cross-cutting concerns (config, db, valkey, boundary
//!   errors, health, telemetry, server shell, middleware). Feature-agnostic by
//!   rule: platform may never import features.
//! * [`features`] — per-feature clean-architecture slices:
//!   `contract` (inbound wire types, leaf) · `domain` (entities + errors,
//!   innermost) · `port` (outbound ports) · `application` (use cases, speaks
//!   contract types at the boundary) · `adapter/driving` + `adapter/driven`
//!   (interface adapters) · `di` (composition).
//! * [`api`] — published contract facade for external consumers. Internal code
//!   must not import it: the dependency is strictly outward-only.
//! * [`app`] — the composition root.
//!
//! All dependencies point inward (business logic never knows about
//! databases or frameworks), and the rule is enforced executably by
//! `tests/architecture.rs`.

pub mod api;
pub mod app;
pub mod features;
pub mod platform;

pub use app::{Built, build};
pub use platform::config::Config;
pub use platform::telemetry::Telemetry;

/// Top-level run entry point. Loads config from the environment, validates it,
/// then boots the full HTTP + gRPC server stack and waits for a shutdown signal.
///
/// Tests that need to run the application against real infrastructure call this
/// directly; the binary `server` (`src/main.rs`) is a thin wrapper.
pub async fn run() -> anyhow::Result<()> {
    let cfg = platform::config::Config::load()?;
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
