//! Composition point for the sales feature (Go `sales/di/di.go` parity).
//! Warn-only stub this wave; sentinel mapping registered now.
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

/// Wire the sales feature standalone. Gated on `sales.enabled`.
pub fn register(cfg: &Config, _db: PgPool) -> Wired {
    if !cfg.sales.enabled {
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

/// Registry entry point (see [`crate::features::registry::RegisterFn`]).
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    _db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.sales.enabled {
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

/// Embedded migrations this feature owns (Go `Migrations fs.FS` parity): sales
/// owns version 3.
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./src/features/sales/adapter/driven/postgres/migrations")
        .iter()
        .cloned()
        .collect()
}

fn warn_unwired() {
    tracing::warn!("sales feature IO edge (postgres/http/grpc) not yet wired");
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
sales: {{ enabled: {enabled} }}
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
        assert_eq!(code(Error::MachineNotFound), errcodes::NOT_FOUND);
        assert_eq!(code(Error::OutOfStock), errcodes::CONFLICT);
        assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::UnsupportedCoin), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InsufficientPayment), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::ExactChangeRequired), errcodes::INVALID_INPUT);
    }

    #[test]
    fn migrations_own_version_three() {
        let m = migrations();
        assert!(m.iter().any(|m| m.version == 3), "sales owns version 3");
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
