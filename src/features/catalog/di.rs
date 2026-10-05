//! Composition point for the catalog feature (Go `catalog/di/di.go` parity).
//! This wave the IO edge is a stub: `register` / `register_with_grpc` honor the
//! `catalog.enabled` gate, register the domain sentinels, then warn that the
//! postgres + HTTP/gRPC edge is not yet wired. The next wave replaces the
//! warning with adapter construction.
//!
//! Sentinel mapping (Go `apperrors.RegisterSentinel(domain.ErrX, apperrors.ErrY)`,
//! port-spec §3) is the crate-global [`From<Error> for AppError`] impl at this
//! composition edge, so the domain layer stays dependency-free.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::catalog::domain::Error;
use crate::platform::config::Config;
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

/// Catalog sentinel → boundary error mapping (Go registration order §3):
/// `ErrProductNotFound -> NOT_FOUND`; the other three → `INVALID_INPUT`.
///
/// Inert until an adapter can produce a `domain::Error`; registered now so the
/// next wave's adapters can rely on `AppError::from`.
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

/// Wire the catalog feature standalone. Gated on `catalog.enabled` (Go §5.1i):
/// a disabled feature registers nothing.
pub fn register(cfg: &Config, _db: PgPool) -> Wired {
    if !cfg.catalog.enabled {
        return Wired {
            http: Router::new(),
            grpc: crate::platform::server::empty_grpc_router(),
        };
    }
    warn_unwired();
    Wired {
        http: Router::new(),
        grpc: crate::platform::server::empty_grpc_router(),
    }
}

/// Registry entry point (see [`crate::features::registry::RegisterFn`]). Gated
/// on `catalog.enabled`; disabled leaves the accumulating gRPC router untouched.
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    _db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.catalog.enabled {
        return Wired {
            http: Router::new(),
            grpc,
        };
    }
    warn_unwired();
    Wired {
        http: Router::new(),
        grpc,
    }
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

fn warn_unwired() {
    tracing::warn!("catalog feature IO edge (postgres/http/grpc) not yet wired");
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
    async fn enabled_feature_still_registers_no_routes_this_wave() {
        let wired = register(&cfg_enabled(true), lazy_pool());
        assert!(!wired.http.has_routes(), "IO edge is a stub this wave");
    }
}
