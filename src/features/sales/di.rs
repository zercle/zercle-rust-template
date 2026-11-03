//! Composition point for the sales feature (Go `sales/di/di.go` parity,
//! port-spec §5.3i): postgres repository → use case → HTTP + gRPC adapters,
//! gated on `sales.enabled`.
//!
//! Sentinel mapping (Go §3, registration order):
//! `ErrProductNotFound`, `ErrMachineNotFound` → `NOT_FOUND`;
//! `ErrOutOfStock` → `CONFLICT`; `ErrInvalidId`, `ErrUnsupportedCoin`,
//! `ErrInsufficientPayment`, `ErrExactChangeRequired` → `INVALID_INPUT`.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::sales::domain::Error;
use crate::features::sales::handler::{grpc, http};
use crate::features::sales::repository::postgres::PgRepository;
use crate::features::sales::usecase::{Service, Usecase};
use crate::platform::config::Config;
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::ProductNotFound | Error::MachineNotFound => AppError::NotFound { cause: None },
            Error::OutOfStock => AppError::Conflict,
            Error::InvalidId
            | Error::UnsupportedCoin
            | Error::InsufficientPayment
            | Error::ExactChangeRequired => AppError::InvalidInput { cause: None },
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Wire the sales feature standalone: postgres repository → use case → HTTP +
/// gRPC adapters, empty platform gRPC router. Mirrors Go `di.Register`; used by
/// tests and one-off callers. Gated on `sales.enabled` (Go §5.3i).
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.sales.enabled {
        return Wired::empty(crate::platform::server::empty_grpc_router());
    }
    build(db, crate::platform::server::empty_grpc_router())
}

/// Registry entry point (see [`crate::features::registry::RegisterFn`]). The
/// `valkey` handle is unused (sales owns no cache) but part of the uniform
/// feature signature. Gated on `sales.enabled`; when disabled contributes no
/// routes and leaves the gRPC router untouched.
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.sales.enabled {
        return Wired::empty(grpc);
    }
    build(db, grpc)
}

fn build(db: PgPool, grpc: GrpcRouter) -> Wired {
    let repo = Arc::new(PgRepository::new(db));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(repo));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

/// Embedded migrations this feature owns (Go `Migrations fs.FS` parity): sales
/// owns version 3.
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/sales/repository/postgres/migrations")
        .iter()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
