//! Composition point for the reporting feature (Go `reporting/di/di.go`
//! parity, port-spec §5.4i). Builds the driven read-only postgres adapter, the
//! use case, and the axum + tonic driving adapters, and registers the domain
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

use crate::features::reporting::adapter::driven::postgres::PgRepository;
use crate::features::reporting::adapter::driving::{grpc, http};
use crate::features::reporting::application::{Service, Usecase};
use crate::features::reporting::domain::Error;
use crate::platform::config::Config;
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
        return empty_wired(crate::platform::server::empty_grpc_router());
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
        return empty_wired(grpc);
    }
    build(cfg, db, grpc)
}

/// A feature contribution that registers nothing (disabled feature).
fn empty_wired(grpc: GrpcRouter) -> Wired {
    Wired {
        http: Router::new(),
        grpc,
    }
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

/// Reporting config is validated `>= 1` u32s; clamp to i32 for the port boundary
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
reporting: {{ enabled: {enabled} }}
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
    fn domain_sentinel_maps_to_invalid_input() {
        assert_eq!(
            AppError::from(Error::InvalidTopMachines).code(),
            errcodes::INVALID_INPUT
        );
    }

    #[test]
    fn domain_internal_maps_to_internal() {
        assert_eq!(
            AppError::from(Error::Internal { cause: None }).code(),
            errcodes::INTERNAL
        );
    }

    #[tokio::test]
    async fn disabled_feature_registers_nothing() {
        let wired = register(&cfg_enabled(false), lazy_pool());
        assert!(!wired.http.has_routes());
    }

    #[tokio::test]
    async fn enabled_feature_registers_summary_route() {
        let wired = register(&cfg_enabled(true), lazy_pool());
        assert!(
            wired.http.has_routes(),
            "enabled reporting mounts GET /api/v1/reports/summary"
        );
    }
}

/// Live-DB integration suite for the reporting read-only edge. Lives at the
/// composition root (the only reporting module allowed to import both the
/// driven adapter and the application layer); the architecture test forbids
/// `adapter/driven` from knowing the application layer.
#[cfg(test)]
mod integration {
    use once_cell::sync::Lazy;
    use sqlx::PgPool;
    use sqlx::postgres::PgPoolOptions;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use crate::features::reporting::adapter::driven::postgres::PgRepository;
    use crate::features::reporting::application::{Service, Usecase};
    use crate::features::reporting::contract::SummaryRequest;
    use crate::features::reporting::domain::{Error, Overview};
    use crate::features::reporting::port::Repository;
    use std::sync::Arc;

    /// Serializes the cases: they share one database and TRUNCATE each other's
    /// tables (Go runs the integration suite with `-p 1`).
    static DB_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    /// Hard-fail setup: no clean skip. Applies the registry's in-suite
    /// migrations (catalog + machines + sales; reporting owns none) and
    /// truncates every source table before each case.
    async fn setup() -> PgPool {
        let url = crate::platform::db::integration_db_url();
        assert_ne!(
            std::env::var("APP_ENVIRONMENT").as_deref(),
            Ok("production"),
            "integration tests must not run against production environment (APP_ENVIRONMENT=production)"
        );
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect DATABASE_URL (hard fail: integration tests require live postgres)");
        crate::features::registry::migrator()
            .expect("registry migrator builds")
            .run(&pool)
            .await
            .expect("apply in-suite migrations");
        sqlx::query(
            "TRUNCATE TABLE sales_purchases, machines, catalog_products RESTART IDENTITY CASCADE",
        )
        .execute(&pool)
        .await
        .expect("truncate reporting source tables");
        pool
    }

    /// Fresh read-only repository over the live pool.
    fn repo(pool: &PgPool) -> Arc<dyn Repository> {
        Arc::new(PgRepository::new(pool.clone()))
    }

    async fn insert_product(pool: &PgPool, id: Uuid, name: &str, price_cents: i32, stock: i32) {
        sqlx::query(
            "INSERT INTO catalog_products (id, name, price_cents, stock) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(name)
        .bind(price_cents)
        .bind(stock)
        .execute(pool)
        .await
        .expect("insert catalog product");
    }

    async fn insert_machine(pool: &PgPool, id: Uuid, label: &str, coin_bank_json: &str) {
        sqlx::query("INSERT INTO machines (id, label, coin_bank) VALUES ($1, $2, $3::jsonb)")
            .bind(id)
            .bind(label)
            .bind(coin_bank_json)
            .execute(pool)
            .await
            .expect("insert machine");
    }

    async fn insert_purchase(
        pool: &PgPool,
        id: Uuid,
        machine_id: Uuid,
        product_id: Uuid,
        price_cents: i32,
    ) {
        sqlx::query(
            "INSERT INTO sales_purchases \
             (id, machine_id, product_id, price_cents, total_inserted_cents, change_cents, change_coins) \
             VALUES ($1, $2, $3, $4, $4, 0, '[]'::jsonb)",
        )
        .bind(id)
        .bind(machine_id)
        .bind(product_id)
        .bind(price_cents)
        .execute(pool)
        .await
        .expect("insert sales purchase");
    }

    #[ignore = "requires live postgres (DB_* / DATABASE_URL)"]
    #[tokio::test]
    async fn overview_and_top_are_empty_on_empty_database() {
        let _guard = DB_LOCK.lock().await;
        let pool = setup().await;
        let repo = repo(&pool);

        assert_eq!(
            repo.get_overview().await.expect("overview"),
            Overview::default()
        );
        assert!(repo.get_top_machines(5).await.expect("top").is_empty());
    }

    #[ignore = "requires live postgres (DB_* / DATABASE_URL)"]
    #[tokio::test]
    async fn repository_aggregates_totals_and_excludes_salesless_machines() {
        let _guard = DB_LOCK.lock().await;
        let pool = setup().await;

        let p1 = Uuid::from_u128(0x100);
        let p2 = Uuid::from_u128(0x101);
        insert_product(&pool, p1, "cola", 100, 10).await;
        insert_product(&pool, p2, "chips", 200, 5).await;

        let m_zero = Uuid::from_u128(0x200);
        let m_a = Uuid::from_u128(0x201);
        let m_b = Uuid::from_u128(0x202);
        insert_machine(&pool, m_zero, "zero-sales", r#"{"25":4}"#).await;
        insert_machine(&pool, m_a, "lobby", r#"{"25":2,"100":1}"#).await;
        insert_machine(&pool, m_b, "cafe", r#"{"5":3}"#).await;

        insert_purchase(&pool, Uuid::from_u128(0x300), m_a, p1, 100).await;
        insert_purchase(&pool, Uuid::from_u128(0x301), m_a, p2, 200).await;
        insert_purchase(&pool, Uuid::from_u128(0x302), m_b, p1, 50).await;

        let repo = repo(&pool);
        let overview = repo.get_overview().await.expect("overview");
        assert_eq!(overview.product_count, 2);
        assert_eq!(overview.total_stock, 15);
        assert_eq!(overview.machine_count, 3);
        assert_eq!(overview.total_bank_cents, 265);
        assert_eq!(overview.purchase_count, 3);
        assert_eq!(overview.revenue_cents, 350);

        let top = repo.get_top_machines(5).await.expect("top");
        assert_eq!(
            top.len(),
            2,
            "zero-sales machine must be excluded by the INNER JOIN"
        );
        assert_eq!(top[0].machine_id, m_a);
        assert_eq!(top[0].purchase_count, 2);
        assert_eq!(top[0].revenue_cents, 300);
        assert_eq!(top[1].machine_id, m_b);
        assert_eq!(top[1].purchase_count, 1);
        assert_eq!(top[1].revenue_cents, 50);
    }

    #[ignore = "requires live postgres (DB_* / DATABASE_URL)"]
    #[tokio::test]
    async fn top_machines_tie_break_by_machine_id_ascending() {
        let _guard = DB_LOCK.lock().await;
        let pool = setup().await;

        let product = Uuid::from_u128(0x400);
        insert_product(&pool, product, "cola", 100, 1).await;

        let m_low = Uuid::from_u128(0x001);
        let m_high = Uuid::from_u128(0x999);
        insert_machine(&pool, m_low, "low", "{}").await;
        insert_machine(&pool, m_high, "high", "{}").await;
        insert_purchase(&pool, Uuid::from_u128(0x401), m_low, product, 100).await;
        insert_purchase(&pool, Uuid::from_u128(0x402), m_high, product, 100).await;

        let repo = repo(&pool);
        let top = repo.get_top_machines(20).await.expect("top");
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].revenue_cents, top[1].revenue_cents, "tie pinned");
        assert_eq!(top[0].purchase_count, top[1].purchase_count, "tie pinned");
        assert_eq!(top[0].machine_id, m_low, "lower id wins the tie");
        assert_eq!(top[1].machine_id, m_high);
    }

    #[ignore = "requires live postgres (DB_* / DATABASE_URL)"]
    #[tokio::test]
    async fn usecase_clamps_default_and_zero_but_rejects_over_max() {
        let _guard = DB_LOCK.lock().await;
        let pool = setup().await;

        let product = Uuid::from_u128(0x500);
        insert_product(&pool, product, "cola", 100, 1).await;
        for i in 0..6u32 {
            let mid = Uuid::from_u128(0x600 + u128::from(i));
            insert_machine(&pool, mid, &format!("m{i}"), "{}").await;
            let price = (i as i32 + 1) * 100;
            insert_purchase(
                &pool,
                Uuid::from_u128(0x700 + u128::from(i)),
                mid,
                product,
                price,
            )
            .await;
        }

        let repo = repo(&pool);
        let svc = Usecase::new(repo, 5, 20);

        let defaulted = svc
            .summary(SummaryRequest { top: None })
            .await
            .expect("default");
        assert_eq!(
            defaulted.top_machines.len(),
            5,
            "absent top clamps to default"
        );
        assert_eq!(defaulted.top_machines[0].revenue_cents, 600);

        let zero = svc
            .summary(SummaryRequest { top: Some(0) })
            .await
            .expect("zero");
        assert_eq!(zero.top_machines.len(), 5, "top=0 clamps to default");

        let err = svc
            .summary(SummaryRequest { top: Some(21) })
            .await
            .expect_err("top above max is rejected before any repository call");
        assert_eq!(err, Error::InvalidTopMachines);
    }
}
