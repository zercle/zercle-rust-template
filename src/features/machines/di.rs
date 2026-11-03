//! Composition point for the machines feature (Go `machines/di/di.go` parity,
//! port-spec §5.2i): builds the postgres repository, the use case, and the
//! handlers, and registers the domain sentinel → boundary error mapping.
//!
//! Sentinel mapping (Go §3, registration order):
//! `ErrMachineNotFound -> NOT_FOUND`; `ErrInvalidId`,
//! `ErrInvalidMachineLabel`, `ErrUnsupportedCoin` → `INVALID_INPUT`.
//!
//! The feature registry ([`crate::features::registry`]) drives
//! [`register_with_grpc`]; [`register`] is the standalone convenience path used
//! by tests that only need the HTTP router.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::machines::domain::Error;
use crate::features::machines::handler::{grpc, http};
use crate::features::machines::repository::postgres::PgRepository;
use crate::features::machines::usecase::{Service, Usecase};
use crate::platform::config::{Config, clamp_i32};
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::MachineNotFound => AppError::NotFound { cause: None },
            Error::InvalidId | Error::InvalidMachineLabel | Error::UnsupportedCoin => {
                AppError::InvalidInput { cause: None }
            }
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Wire the machines feature standalone: postgres repository → use case → HTTP
/// and gRPC adapters, starting from an empty platform gRPC router. Mirrors Go
/// `di.Register`; used by tests and one-off callers.
///
/// Gated on `machines.enabled` (Go §5.2i): a disabled feature registers nothing.
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.machines.enabled {
        return Wired::empty(crate::platform::server::empty_grpc_router());
    }
    build(cfg, db, crate::platform::server::empty_grpc_router())
}

/// Registry entry point (see [`crate::features::registry::RegisterFn`]): wire
/// the feature onto the accumulating gRPC router the registry threads through
/// every feature. `valkey` is unused (machines owns no cache); it is part of
/// the uniform feature signature.
///
/// Gated on `machines.enabled` (Go §5.2i): when disabled it contributes no
/// routes and leaves the gRPC router untouched.
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.machines.enabled {
        return Wired::empty(grpc);
    }
    build(cfg, db, grpc)
}

/// Embedded migrations this feature owns (Go `Migrations fs.FS` parity):
/// machines owns version 2.
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/machines/repository/postgres/migrations")
        .iter()
        .cloned()
        .collect()
}

fn build(cfg: &Config, db: PgPool, grpc: GrpcRouter) -> Wired {
    let repo = Arc::new(PgRepository::new(db));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        clamp_i32(cfg.machines.default_page_size),
        clamp_i32(cfg.machines.max_page_size),
        clamp_i32(cfg.machines.max_label_length),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

#[cfg(test)]
mod tests;
