//! Composition point for the catalog feature (Go `catalog/di/di.go` parity).
//! Honors the `catalog.enabled` gate (a disabled feature registers nothing),
//! builds the driven adapter (`PgRepository`, wrapped in the cache-aside
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

use crate::features::catalog::adapter::driven::cached::CachedRepository;
use crate::features::catalog::adapter::driven::postgres::PgRepository;
use crate::features::catalog::adapter::driving::{grpc, http};
use crate::features::catalog::application::{Service, Usecase};
use crate::features::catalog::domain::Error;
use crate::features::catalog::port::Repository;
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
        return empty_wired(crate::platform::server::empty_grpc_router());
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
        return empty_wired(grpc);
    }
    let postgres: Arc<dyn Repository> = Arc::new(PgRepository::new(db));
    let repo: Arc<dyn Repository> =
        Arc::new(CachedRepository::new(postgres, valkey, cfg.valkey_ttl()));
    build(cfg, repo, grpc)
}

/// A disabled feature contributes nothing (empty HTTP router, gRPC router
/// untouched) — Go's `di.Register` returning early.
fn empty_wired(grpc: GrpcRouter) -> Wired {
    Wired {
        http: Router::new(),
        grpc,
    }
}

/// Build the use case + driving adapters for an already-chosen repository.
fn build(cfg: &Config, repo: Arc<dyn Repository>, grpc: GrpcRouter) -> Wired {
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        clamp_i32(cfg.catalog.default_page_size),
        clamp_i32(cfg.catalog.max_page_size),
        clamp_i32(cfg.catalog.max_name_length),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

/// Config limits are `u32`; the use case speaks `i32`. Saturate rather than
/// wrap (validation bounds these well below `i32::MAX` anyway).
fn clamp_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// Embedded migrations this feature owns, in the single global version
/// namespace (Go `Migrations fs.FS` parity, port-spec §1): catalog owns
/// version 1. `sqlx::migrate!` yields one entry per `.sql` file (up + down).
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/catalog/adapter/driven/postgres/migrations")
        .iter()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::errors::errcodes;
    use sqlx::postgres::PgPoolOptions;

    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/none")
            .expect("lazy pool")
    }

    fn cfg_enabled(enabled: bool) -> Config {
        let yaml = format!(
            r#"
app: {{ name: t, environment: dev, host: 0.0.0.0, port: 8080 }}
http: {{ host: 0.0.0.0, port: 8080, body_limit: "1M" }}
grpc: {{ host: 0.0.0.0, port: 50051 }}
db: {{ host: localhost, port: 5432, name: app, user: postgres, password: postgres, ssl_mode: disable, max_conns: 1, min_conns: 0 }}
valkey: {{ host: localhost, port: 6379, db: 0 }}
otel: {{ exporter: none, service_name: t, sampling: 1.0 }}
log: {{ level: info, format: json }}
catalog: {{ enabled: {enabled} }}
"#
        );
        ::config::Config::builder()
            .add_source(::config::File::from_str(&yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("build")
            .try_deserialize()
            .expect("deserialize")
    }

    #[test]
    fn domain_sentinels_map_to_boundary_codes() {
        let code = |e: Error| AppError::from(e).code().to_string();
        assert_eq!(code(Error::ProductNotFound), errcodes::NOT_FOUND);
        assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InvalidProductName), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InvalidPrice), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::Internal { cause: None }), errcodes::INTERNAL);
    }

    #[test]
    fn migrations_own_version_one() {
        let m = migrations();
        assert!(m.iter().any(|m| m.version == 1), "catalog owns version 1");
    }

    #[tokio::test]
    async fn disabled_feature_registers_nothing() {
        let wired = register(&cfg_enabled(false), lazy_pool());
        assert!(!wired.http.has_routes());
    }

    #[tokio::test]
    async fn enabled_feature_registers_http_routes() {
        // The lazy pool never connects; `register` only constructs adapters.
        let wired = register(&cfg_enabled(true), lazy_pool());
        assert!(wired.http.has_routes(), "enabled catalog mounts /api/v1");
    }
}
