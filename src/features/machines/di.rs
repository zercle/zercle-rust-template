//! Composition point for the machines feature (Go `machines/di/di.go` parity,
//! port-spec §5.2i): builds the driven adapter, the use case, and the driving
//! adapters, and registers the domain sentinel → boundary error mapping.
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

use crate::features::machines::adapter::driven::postgres::PgRepository;
use crate::features::machines::adapter::driving::{grpc, http};
use crate::features::machines::application::{Service, Usecase};
use crate::features::machines::domain::Error;
use crate::platform::config::Config;
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
        return Wired {
            http: Router::new(),
            grpc: crate::platform::server::empty_grpc_router(),
        };
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
        return Wired {
            http: Router::new(),
            grpc,
        };
    }
    build(cfg, db, grpc)
}

/// Embedded migrations this feature owns (Go `Migrations fs.FS` parity):
/// machines owns version 2.
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/machines/adapter/driven/postgres/migrations")
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

/// Config limits are validated `>= 1` u32s; clamp to i32 for the port boundary
/// (Go carries them as int32).
fn clamp_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
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
machines: {{ enabled: {enabled} }}
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
        assert_eq!(code(Error::MachineNotFound), errcodes::NOT_FOUND);
        assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InvalidMachineLabel), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::UnsupportedCoin), errcodes::INVALID_INPUT);
    }

    #[test]
    fn migrations_own_version_two() {
        let m = migrations();
        assert!(m.iter().any(|m| m.version == 2), "machines owns version 2");
    }

    #[tokio::test]
    async fn disabled_feature_registers_nothing() {
        let wired = register(&cfg_enabled(false), lazy_pool());
        assert!(!wired.http.has_routes());
    }

    #[tokio::test]
    async fn enabled_feature_registers_routes() {
        let wired = register(&cfg_enabled(true), lazy_pool());
        assert!(
            wired.http.has_routes(),
            "enabled feature mounts its /api/v1 routes"
        );
    }
}
