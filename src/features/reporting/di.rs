//! Composition point for the reporting feature (Go `reporting/di/di.go`
//! parity, port-spec §5.4i). Builds the read-only postgres repository, the
//! use case, and the axum + tonic handlers, and registers the domain
//! sentinel → boundary error mapping.
//!
//! The feature registry ([`crate::features::registry`]) drives
//! [`register_with_grpc`]; [`register`] is the standalone convenience path used
//! by tests that only need the HTTP router.
//!
//! Sentinel mapping (Go §3): `ErrInvalidTopMachines -> INVALID_INPUT`.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::reporting::domain::Error;
use crate::features::reporting::handler::{grpc, http};
use crate::features::reporting::repository::postgres::PgRepository;
use crate::features::reporting::usecase::{Service, Usecase};
use crate::platform::config::{Config, clamp_i32};
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

/// Sentinel → boundary error mapping, defined at this composition edge so the
/// domain layer stays dependency-free (crate-global [`From`] impl).
impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidTopMachines => AppError::InvalidInput { cause: None },
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Wire the reporting feature standalone: read-only postgres repository → use
/// case → HTTP + gRPC adapters, starting from an empty platform gRPC router.
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.reporting.enabled {
        return Wired::empty(crate::platform::server::empty_grpc_router());
    }
    build(cfg, db, crate::platform::server::empty_grpc_router())
}

/// Registry entry point (see [`crate::features::registry::RegisterFn`]). Wires
/// the feature onto the accumulating gRPC router the registry threads through
/// every feature. `valkey` is unused (reporting owns no cache) but is part of
/// the uniform feature signature.
///
/// Reporting owns no schema, so the registry contributes no migrations for it
/// (Go `Migrations` omitted, port-spec §1).
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.reporting.enabled {
        return Wired::empty(grpc);
    }
    build(cfg, db, grpc)
}

fn build(cfg: &Config, db: PgPool, grpc: GrpcRouter) -> Wired {
    let repo = Arc::new(PgRepository::new(db));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        clamp_i32(cfg.reporting.default_top_machines),
        clamp_i32(cfg.reporting.max_top_machines),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration;
