//! Composition point for the catalog feature (Go `catalog/di/di.go` parity).
//! Honors the `catalog.enabled` gate (a disabled feature registers nothing),
//! builds the postgres repository (`PgRepository`, wrapped in the cache-aside
//! `CachedRepository` when the registry passes the Valkey connection), the
//! use case with the configured limits, and mounts the axum + tonic driving
//! adapters under `/api/v1`.
//!
//! Sentinel mapping (Go `apperrors.RegisterSentinel(domain.ErrX, apperrors.ErrY)`,
//! port-spec §3) is the crate-global [`From<Error> for AppError`] impl at this
//! composition edge, so the domain layer stays dependency-free.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::catalog::domain::Error;
use crate::features::catalog::handler::{grpc, http};
use crate::features::catalog::repository::Repository;
use crate::features::catalog::repository::postgres::PgRepository;
use crate::features::catalog::repository::postgres::cached::CachedRepository;
use crate::features::catalog::usecase::{Service, Usecase};
use crate::platform::config::Config;
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

/// Catalog sentinel → boundary error mapping (Go registration order §3):
/// `ErrProductNotFound -> NOT_FOUND`; the other three → `INVALID_INPUT`.
impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::ProductNotFound => AppError::NotFound { cause: None },
            Error::InvalidId | Error::InvalidProductName | Error::InvalidPrice => {
                AppError::InvalidInput { cause: None }
            }
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Wire the catalog feature standalone (no cache-aside): postgres repository →
/// use case → HTTP + gRPC adapters, starting from an empty platform gRPC
/// router. Gated on `catalog.enabled` (Go §5.1i): a disabled feature registers
/// nothing.
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.catalog.enabled {
        return Wired::empty(crate::platform::server::empty_grpc_router());
    }
    let repo: Arc<dyn Repository> = Arc::new(PgRepository::new(db));
    build(cfg, repo, crate::platform::server::empty_grpc_router())
}

/// Registry entry point (see [`crate::features::registry::RegisterFn`]). Gated
/// on `catalog.enabled`; disabled leaves the accumulating gRPC router untouched.
///
/// When enabled, wraps the postgres repository in the cache-aside
/// [`CachedRepository`] over the shared Valkey connection (Go §5.1i: the
/// registry-provided cache-aside client decorates the repository).
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    db: PgPool,
    valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.catalog.enabled {
        return Wired::empty(grpc);
    }
    let postgres: Arc<dyn Repository> = Arc::new(PgRepository::new(db));
    let repo: Arc<dyn Repository> =
        Arc::new(CachedRepository::new(postgres, valkey, cfg.valkey_ttl()));
    build(cfg, repo, grpc)
}

/// Build the use case + handlers for an already-chosen repository.
fn build(cfg: &Config, repo: Arc<dyn Repository>, grpc: GrpcRouter) -> Wired {
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        crate::platform::config::clamp_i32(cfg.catalog.default_page_size),
        crate::platform::config::clamp_i32(cfg.catalog.max_page_size),
        crate::platform::config::clamp_i32(cfg.catalog.max_name_length),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

/// Embedded migrations this feature owns, in the single global version
/// namespace (Go `Migrations fs.FS` parity, port-spec §1): catalog owns
/// version 1. `sqlx::migrate!` yields one entry per `.sql` file (up + down).
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/catalog/repository/postgres/migrations")
        .iter()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
