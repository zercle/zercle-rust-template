//! Integration test for the example feature's HTTP routes against a real
//! Postgres + Valkey pair.
//!
//! Skips cleanly (`return Ok(())`) when neither backing service is reachable,
//! so `cargo test --test example_http` is green on a developer machine without
//! docker-compose running and still exercises the full HTTP path against a
//! real DB when it is.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode as SC};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use zercle_rust_template::config::Config;
use zercle_rust_template::features::example::{
    PgRepository, ServiceImpl, http_routes as example_http_routes,
};

/// Happy path + error path against a real Postgres: build a pool, run
/// migrations, mount the example router, and exercise POST/GET /items.
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

    let repo: Arc<dyn zercle_rust_template::features::example::Repository> =
        Arc::new(PgRepository::new(pool.clone()));
    let service = Arc::new(ServiceImpl::new(
        repo,
        cfg.example.default_page_size as i32,
        cfg.example.max_page_size as i32,
        cfg.example.max_name_length as i32,
    ));

    let app = example_http_routes(service);

    // --- Happy path: POST /items → 201 -------------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"alpha"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::CREATED, "POST /items happy path");
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let created_id = v["id"].as_str().expect("id is a string").to_string();
    assert!(Uuid::parse_str(&created_id).is_ok(), "id is a valid uuid");
    assert_eq!(v["name"], "alpha");

    // --- Happy path: GET /items → 200 + the row we just created ------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::OK, "GET /items happy path");
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = v["items"].as_array().unwrap();
    assert!(
        items.iter().any(|it| it["name"] == "alpha"),
        "alpha present in list: {v}"
    );

    // --- GET /items/:id → 200 hit -----------------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/items/{created_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::OK, "GET /items/:id hit");

    // --- POST /items → 400 invalid name (empty) ---------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::BAD_REQUEST, "POST empty name");

    // --- GET /items/:id → 404 not found ------------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/items/{}", Uuid::nil()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::NOT_FOUND, "GET missing id");

    // --- GET /items/:id → 400 bad uuid -------------------------------
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/items/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), SC::BAD_REQUEST, "GET bad uuid");

    Ok(())
}
