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

use crate::features::sales::adapter::driven::postgres::PgRepository;
use crate::features::sales::adapter::driving::{grpc, http};
use crate::features::sales::application::{Service, Usecase};
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

/// Wire the sales feature standalone: postgres repository → use case → HTTP +
/// gRPC adapters, empty platform gRPC router. Mirrors Go `di.Register`; used by
/// tests and one-off callers. Gated on `sales.enabled` (Go §5.3i).
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    if !cfg.sales.enabled {
        return Wired {
            http: Router::new(),
            grpc: crate::platform::server::empty_grpc_router(),
        };
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
        return Wired {
            http: Router::new(),
            grpc,
        };
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
    sqlx::migrate!("./src/features/sales/adapter/driven/postgres/migrations")
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
    async fn enabled_feature_registers_the_purchases_route() {
        let wired = register(&cfg_enabled(true), lazy_pool());
        assert!(wired.http.has_routes(), "IO edge wires POST /purchases");
    }

    // --- Live-DB end-to-end integration tests ----------------------------
    //
    // Gated `#[ignore]` so `cargo test` is green without infra. Run with:
    //   DATABASE_URL=postgres://…it_sales… cargo test --lib sales -- --include-ignored

    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use sqlx::Connection;
    use sqlx::postgres::PgConnection;
    use tower::ServiceExt;
    use uuid::Uuid;

    // Same advisory-lock key as the postgres adapter's integration suite: both
    // share one physical database, so they must serialize and TRUNCATE safely.
    const TEST_LOCK_KEY: i64 = 0x5a1e_55a1e5;

    async fn test_pool() -> (PgPool, PgConnection) {
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL required for sales integration tests");
        if std::env::var("APP_ENVIRONMENT").as_deref() == Ok("production") || url.contains("/prod")
        {
            panic!("integration tests must not run against production");
        }
        let mut lock = PgConnection::connect(&url)
            .await
            .expect("connect it_sales for advisory lock");
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(TEST_LOCK_KEY)
            .execute(&mut lock)
            .await
            .expect("acquire test advisory lock");
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .expect("connect it_sales");
        crate::features::registry::migrator()
            .expect("build merged migrator")
            .run(&pool)
            .await
            .expect("apply registry migrations");
        sqlx::query(
            "TRUNCATE TABLE sales_purchases, machines, catalog_products RESTART IDENTITY CASCADE",
        )
        .execute(&pool)
        .await
        .expect("truncate sales tables");
        (pool, lock)
    }

    async fn seed(pool: &PgPool, machine: Uuid, product: Uuid, price: i32, stock: i32) {
        sqlx::query(
            "INSERT INTO catalog_products (id, name, price_cents, stock) VALUES ($1, 'widget', $2, $3)",
        )
        .bind(product)
        .bind(price)
        .bind(stock)
        .execute(pool)
        .await
        .expect("seed product");
        sqlx::query("INSERT INTO machines (id, label, coin_bank) VALUES ($1, 'lobby', $2::jsonb)")
            .bind(machine)
            .bind(r#"{"25":3,"50":1}"#)
            .execute(pool)
            .await
            .expect("seed machine");
    }

    async fn post_purchase(app: Router, body: &str) -> (SC, serde_json::Value) {
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/purchases")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[ignore]
    #[tokio::test]
    async fn sales_wired_purchase_overpay_returns_201_and_commits() {
        let (pool, _lock) = test_pool().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed(&pool, machine, product, 25, 5).await;

        let app = register(&cfg_enabled(true), pool.clone()).http;
        let body =
            format!(r#"{{"machine_id":"{machine}","product_id":"{product}","coins":[100]}}"#);
        let (status, json) = post_purchase(app, &body).await;
        assert_eq!(status, SC::CREATED, "body: {json}");
        assert_eq!(json["change_cents"], 75);
        assert_eq!(json["change_coins"], serde_json::json!([50, 25]));

        let stock: i32 = sqlx::query_scalar("SELECT stock FROM catalog_products WHERE id = $1")
            .bind(product)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stock, 4, "stock decremented");
        let sales: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sales_purchases")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(sales, 1, "sale row inserted");
    }

    #[ignore]
    #[tokio::test]
    async fn sales_wired_insufficient_stock_maps_to_conflict_envelope() {
        let (pool, _lock) = test_pool().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed(&pool, machine, product, 25, 0).await;

        let app = register(&cfg_enabled(true), pool.clone()).http;
        let body = format!(r#"{{"machine_id":"{machine}","product_id":"{product}","coins":[25]}}"#);
        let (status, json) = post_purchase(app, &body).await;
        assert_eq!(status, SC::CONFLICT);
        assert_eq!(json["error"], "CONFLICT");
        assert_eq!(json["message"], "conflict");
    }

    #[ignore]
    #[tokio::test]
    async fn sales_wired_unsupported_coin_maps_to_invalid_input_envelope() {
        let (pool, _lock) = test_pool().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed(&pool, machine, product, 25, 5).await;

        let app = register(&cfg_enabled(true), pool.clone()).http;
        let body = format!(r#"{{"machine_id":"{machine}","product_id":"{product}","coins":[7]}}"#);
        let (status, json) = post_purchase(app, &body).await;
        assert_eq!(status, SC::BAD_REQUEST);
        assert_eq!(json["error"], "INVALID_INPUT");
    }
}
