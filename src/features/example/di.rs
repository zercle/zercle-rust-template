//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Composition point for the example feature (Go `di.Register` parity): builds
//! the driven adapter, the use case, and the driving adapters, and registers
//! the domain sentinel → boundary error mapping.
//!
//! The feature registry ([`crate::features::registry`]) drives
//! [`register_with_grpc`]; [`register`] is the standalone convenience path used
//! by tests that only need the HTTP router.

use std::sync::Arc;

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::features::example::adapter::driven::postgres::PgRepository;
use crate::features::example::adapter::driving::{grpc, http};
use crate::features::example::application::{Service, Usecase};
use crate::features::example::domain::Error;
use crate::platform::config::Config;
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;
pub use crate::platform::server::Wired;

/// Sentinel → boundary error mapping, registered here at the composition edge
/// so the domain layer stays dependency-free (Go
/// `apperrors.RegisterSentinel(domain.ErrX, apperrors.ErrY)` parity; the impl
/// is crate-global once defined, so every adapter can rely on `AppError::from`).
impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotFound => AppError::NotFound { cause: None },
            Error::InvalidName | Error::InvalidId => AppError::InvalidInput { cause: None },
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Wire the example feature standalone: postgres repository → use case → HTTP +
/// gRPC adapters, starting from an empty platform gRPC router. Mirrors Go
/// `di.Register`; used by tests and one-off callers.
///
/// Gated on `example.enabled` (Go §5i): a disabled feature registers nothing.
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.example.enabled {
        return empty_wired(crate::platform::server::empty_grpc_router());
    }
    build(cfg, db, crate::platform::server::empty_grpc_router())
}

/// Registry entry point: wire the feature onto the accumulating gRPC router the
/// registry threads through every feature. `valkey` is unused today (the
/// example feature owns no cache); it is part of the uniform feature signature
/// (Go features resolve the cache-aside client from the injector).
///
/// Gated on `example.enabled` (Go §5i): when disabled it contributes no routes
/// and leaves the gRPC router untouched.
pub fn register_with_grpc(
    cfg: &Arc<Config>,
    db: PgPool,
    _valkey: ConnectionManager,
    grpc: GrpcRouter,
) -> Wired {
    if !cfg.example.enabled {
        return empty_wired(grpc);
    }
    build(cfg, db, grpc)
}

/// A feature contribution that registers nothing (disabled feature / Go's
/// `di.Register` returning nil).
fn empty_wired(grpc: GrpcRouter) -> Wired {
    Wired {
        http: Router::new(),
        grpc,
    }
}

/// Embedded migrations this feature owns, in the single global version
/// namespace (Go `Migrations fs.FS` parity, port-spec §1). The example feature
/// still owns the template's top-level `./migrations` directory.
///
/// `sqlx::migrate!` over a directory yields one entry per `.sql` file, so a
/// reversible version contributes both its up and down entries; both are kept
/// so the runner can apply (`up`) and revert (`down`).
pub fn migrations() -> Vec<sqlx::migrate::Migration> {
    sqlx::migrate!("./migrations").iter().cloned().collect()
}

fn build(cfg: &Config, db: PgPool, grpc: GrpcRouter) -> Wired {
    let repo = Arc::new(PgRepository::new(db));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        clamp_i32(cfg.example.default_page_size),
        clamp_i32(cfg.example.max_page_size),
        clamp_i32(cfg.example.max_name_length),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = grpc.add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

/// Config page sizes are validated `>= 1` u32s; clamp to i32 for the proto /
/// port boundary (Go carries them as int32).
fn clamp_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::errors::errcodes;

    #[test]
    fn domain_sentinels_map_to_boundary_codes() {
        // Assert the registered sentinel mapping (the From impl above) lands
        // on the published error codes.
        let code = |e: Error| AppError::from(e).code().to_string();
        assert_eq!(code(Error::NotFound), errcodes::NOT_FOUND);
        assert_eq!(code(Error::InvalidName), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::Internal { cause: None }), errcodes::INTERNAL);
    }

    #[test]
    fn migrations_entry_supplies_the_top_level_dir() {
        let m = migrations();
        assert!(!m.is_empty(), "example feature should own migrations");
        assert!(m.iter().any(|m| m.version == 1));
    }

    #[tokio::test]
    async fn disabled_feature_registers_no_routes() {
        // Go §5i parity: `enabled: false` -> `di.Register` returns early with
        // nothing registered (empty HTTP router, gRPC router untouched).
        let yaml = r#"
app: { name: t, environment: dev, host: 0.0.0.0, port: 8080 }
http: { host: 0.0.0.0, port: 8080, body_limit: "1M" }
grpc: { host: 0.0.0.0, port: 50051 }
db: { host: localhost, port: 5432, name: app, user: postgres, password: postgres, ssl_mode: disable, max_conns: 1, min_conns: 0 }
valkey: { host: localhost, port: 6379, db: 0 }
otel: { exporter: none, service_name: t, sampling: 1.0 }
log: { level: info, format: json }
example: { enabled: false }
"#;
        let cfg: Config = ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("build")
            .try_deserialize()
            .expect("deserialize");
        assert!(!cfg.example.enabled);

        // A lazy pool never connects; the disabled path returns before any DB
        // access, so no infrastructure is needed.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/none")
            .expect("lazy pool");
        let wired = register(&cfg, pool);
        assert!(
            !wired.http.has_routes(),
            "disabled feature mounts no routes"
        );
    }
}
