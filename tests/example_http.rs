//! Integration test for the example feature's HTTP routes against a real
//! Postgres + Valkey pair.
//!
//! Exercises the real feature composition path: `features::example::di::register`
//! (repository → use case → HTTP adapter), with the router mounted exactly as
//! the application mounts it (nested under `/api/v1`).
//!
//! Skips cleanly (`return Ok(())`) when neither backing service is reachable,
//! so `cargo test --test example_http` is green on a developer machine without
//! docker-compose running and still exercises the full HTTP path against a
//! real DB when it is.

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode as SC};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use zercle_rust_template::features::example::di;
use zercle_rust_template::platform::config::Config;

/// Happy path + error path against a real Postgres: build a pool, run
/// migrations, mount the feature via its `di`, and exercise
/// POST/GET `/api/v1/items`.
#[tokio::test]
async fn example_http_round_trip() -> anyhow::Result<()> {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config load failed; skipping: {e:#}");
            return Ok(());
        }
    };
    if !common::infra_reachable(&cfg) {
        eprintln!("infra not reachable; skipping (run `docker compose up -d postgres valkey`)");
        return Ok(());
    }

    let pool = match PgPoolOptions::new()
        .max_connections(cfg.db.max_conns)
        .min_connections(cfg.db.min_conns)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.db_conn_string())
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("postgres unreachable; skipping: {e}");
            return Ok(());
        }
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations apply");

    // Clean any leftover rows so the GET-count assertion is stable.
    sqlx::query("DELETE FROM items").execute(&pool).await.ok();

    let app = di::register(&cfg, pool).http;

    // --- Happy path: POST /api/v1/items → 201 -------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"alpha"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::CREATED, "POST /api/v1/items happy path");
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let created_id = v["id"].as_str().expect("id is a string").to_string();
    assert!(Uuid::parse_str(&created_id).is_ok(), "id is a valid uuid");
    assert_eq!(v["name"], "alpha");

    // --- Happy path: GET /api/v1/items → 200 + the row we just created
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::OK, "GET /api/v1/items happy path");
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = v["items"].as_array().unwrap();
    assert!(
        items.iter().any(|it| it["name"] == "alpha"),
        "alpha present in list: {v}"
    );

    // --- GET /api/v1/items/:id → 200 hit ------------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{created_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::OK, "GET /api/v1/items/:id hit");

    // --- POST /api/v1/items → 400 invalid name (empty) ----------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::BAD_REQUEST, "POST empty name");

    // --- GET /api/v1/items/:id → 404 not found ------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", Uuid::nil()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::NOT_FOUND, "GET missing id");

    // --- GET /api/v1/items/:id → 400 bad uuid -------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/items/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::BAD_REQUEST, "GET bad uuid");

    Ok(())
}
