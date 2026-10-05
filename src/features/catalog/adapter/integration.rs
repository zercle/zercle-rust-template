//! Live-infrastructure integration tests for the catalog IO edge (port-spec
//! §6 test patterns): real Postgres + Valkey, migrations applied in-suite on a
//! throwaway database, TRUNCATE between cases, and a hard failure when the
//! infrastructure is unreachable (no silent skip). A production/remote guard
//! refuses to run against anything but a local database.
//!
//! Ignored by default so `cargo test` stays green without infra. Run with:
//!
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:5432/it_catalog \
//!   cargo test --lib catalog --include-ignored
//! ```
//!
//! The suite uses Valkey DB `15` (FLUSHDB on that db only) so a developer's
//! default DB `0` cache is untouched. Tests serialize on [`INFRA_LOCK`] because
//! they share one database.

use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::catalog::adapter::driven::cached::{
    CachedRepository, DEFAULT_PRODUCT_CACHE_TTL,
};
use crate::features::catalog::adapter::driven::postgres::PgRepository;
use crate::features::catalog::adapter::driving::http;
use crate::features::catalog::application::{Service, Usecase};
use crate::features::catalog::domain::{Error, Product};
use crate::features::catalog::port::Repository;

/// Serializes the shared-database tests (they TRUNCATE each other's tables).
static INFRA_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const LOCAL_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];
/// Isolated Valkey DB for tests (see module docs).
const TEST_VALKEY_DB: u8 = 15;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Production / remote guard: refuse to run against a non-local database.
fn assert_local_environment(db_url: &str, valkey_host: &str) {
    if std::env::var("APP_ENVIRONMENT").as_deref() == Ok("production") {
        panic!("integration tests must not run against production (APP_ENVIRONMENT=production)");
    }
    let db_host = url::Url::parse(db_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string));
    match db_host.as_deref() {
        Some(h) if LOCAL_HOSTS.contains(&h) => {}
        other => panic!(
            "refusing non-local DATABASE_URL host {other:?}; integration tests must target a \
             local throwaway database"
        ),
    }
    if !LOCAL_HOSTS.contains(&valkey_host) {
        panic!(
            "refusing non-local VALKEY_HOST {valkey_host:?}; integration tests must target local \
             infrastructure"
        );
    }
}

fn database_url() -> String {
    std::env::var("DATABASE_URL").expect(
        "hard-fail: DATABASE_URL must point at a local throwaway database (e.g. it_catalog)",
    )
}

fn valkey_url() -> String {
    let host = env_or("VALKEY_HOST", "localhost");
    let port = env_or("VALKEY_PORT", "6379");
    let password = std::env::var("VALKEY_PASSWORD").unwrap_or_default();
    if password.is_empty() {
        format!("redis://{host}:{port}/{TEST_VALKEY_DB}")
    } else {
        format!("redis://:{password}@{host}:{port}/{TEST_VALKEY_DB}")
    }
}

/// Connect to Postgres; hard-fails (never skips) when unreachable.
async fn connect_pool() -> PgPool {
    let url = database_url();
    let valkey_host = env_or("VALKEY_HOST", "localhost");
    assert_local_environment(&url, &valkey_host);
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("hard-fail: local Postgres must be reachable (docker compose up postgres)")
}

/// Connect to the isolated test Valkey DB; hard-fails when unreachable.
async fn connect_valkey() -> ConnectionManager {
    let url = valkey_url();
    let client = redis::Client::open(url.as_str()).expect("build redis client");
    ConnectionManager::new(client)
        .await
        .expect("hard-fail: local Valkey must be reachable (docker compose up valkey)")
}

/// Apply the registry's merged migrations (in-suite) and clear the catalog
/// table so each case starts from a known state.
async fn migrate_and_truncate(pool: &PgPool) {
    crate::features::registry::migrator()
        .expect("build migration set")
        .run(pool)
        .await
        .expect("apply migrations");
    sqlx::query("TRUNCATE TABLE catalog_products RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate catalog_products");
}

fn product(name: &str, price_cents: i32, created_at: OffsetDateTime) -> Product {
    Product {
        id: Uuid::now_v7(),
        name: name.to_string(),
        price_cents,
        stock: 5,
        created_at,
        updated_at: created_at,
    }
}

#[tokio::test]
#[ignore = "requires live Postgres; set DATABASE_URL to a local throwaway database"]
async fn postgres_repository_create_get_list_pagination() {
    let _guard = INFRA_LOCK.lock().await;
    let pool = connect_pool().await;
    migrate_and_truncate(&pool).await;
    let repo = PgRepository::new(pool.clone());

    let base = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    let first = product("oldest", 100, base);
    let second = product("middle", 200, base + time::Duration::seconds(60));
    let third = product("newest", 300, base + time::Duration::seconds(120));
    for p in [&first, &second, &third] {
        repo.create(p).await.expect("create");
    }

    // get by id round-trips the full row.
    let got = repo.get_by_id(second.id).await.expect("get");
    assert_eq!(got, second);

    // Unknown id is a domain not-found (not an infra error).
    assert_eq!(
        repo.get_by_id(Uuid::now_v7()).await.unwrap_err(),
        Error::ProductNotFound
    );

    // Order is created_at DESC; limit=2 returns the two newest, offset=2 the rest.
    let page1 = repo.list(2, 0).await.expect("list page 1");
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].id, third.id);
    assert_eq!(page1[1].id, second.id);

    let page2 = repo.list(2, 2).await.expect("list page 2");
    assert_eq!(page2.len(), 1);
    assert_eq!(page2[0].id, first.id);
}

#[tokio::test]
#[ignore = "requires live Postgres + Valkey; set DATABASE_URL to a local throwaway database"]
async fn cache_aside_miss_then_hit_and_not_found_is_not_cached() {
    let _guard = INFRA_LOCK.lock().await;
    let pool = connect_pool().await;
    migrate_and_truncate(&pool).await;
    let conn = connect_valkey().await;
    let mut flush = conn.clone();
    redis::cmd("FLUSHDB")
        .query_async::<()>(&mut flush)
        .await
        .expect("flush test valkey db");

    let inner = Arc::new(PgRepository::new(pool.clone()));
    let repo = CachedRepository::new(inner, conn.clone(), Duration::from_secs(5));

    let created = product("cached", 250, OffsetDateTime::now_utc());
    repo.create(&created).await.expect("create");

    // Miss -> loads through, writes the cache entry under `catalog:product:<id>`.
    let miss = repo.get_by_id(created.id).await.expect("first get (miss)");
    assert_eq!(miss, created);

    let key = format!("catalog:product:{}", created.id);
    let mut check = conn.clone();
    let stored: Option<String> = check.get(&key).await.expect("read cache key");
    assert!(stored.is_some(), "miss must populate the cache");

    // Remove the row: a second get can only succeed from cache (the hit path).
    sqlx::query("DELETE FROM catalog_products WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await
        .expect("delete row");
    let hit = repo.get_by_id(created.id).await.expect("second get (hit)");
    assert_eq!(hit, created);

    // Not-found is never cached: unknown id -> ProductNotFound, no entry written.
    let unknown = Uuid::now_v7();
    assert_eq!(
        repo.get_by_id(unknown).await.unwrap_err(),
        Error::ProductNotFound
    );
    let unknown_key = format!("catalog:product:{unknown}");
    let cached_not_found: Option<String> = check.get(&unknown_key).await.expect("read unknown key");
    assert!(
        cached_not_found.is_none(),
        "a missing product must not be cached"
    );
}

#[tokio::test]
#[ignore = "requires live Postgres; set DATABASE_URL to a local throwaway database"]
async fn http_get_unknown_id_returns_not_found_envelope() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let _guard = INFRA_LOCK.lock().await;
    let pool = connect_pool().await;
    migrate_and_truncate(&pool).await;

    let repo: Arc<dyn Repository> = Arc::new(PgRepository::new(pool.clone()));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(repo, 20, 100, 255));
    let app = axum::Router::new().nest("/api/v1", http::routes(service));

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/products/{}", Uuid::now_v7()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "NOT_FOUND");
    assert_eq!(json["message"], "resource not found");
}

/// Keeps the spec's default TTL in view; documents the crate divergence where
/// `ValkeyConfig` exposes no `ttl`, so this constant stands in for
/// `cfg.valkey.ttl`.
#[test]
fn default_cache_ttl_matches_spec_default() {
    assert_eq!(DEFAULT_PRODUCT_CACHE_TTL, Duration::from_secs(30));
}
