//! End-to-end test: boot the full application (`run_with_config`) against real
//! Postgres + Valkey and drive one flow per feature through the wired HTTP
//! stack (catalog -> machines -> sales -> reporting), plus the shared probes
//! and one error-envelope assertion.
//!
//! Skips cleanly when the backing services are not reachable. Migrations are
//! assumed already applied (the server never runs them). This mirrors the Go
//! template's `TestServer_EndToEnd`; gRPC is intentionally not probed here —
//! the HTTP flow proves the composition root.

mod common;

use std::time::Duration;

use serde_json::{Value, json};
use tokio::net::TcpListener;

use zercle_rust_template::platform::config::Config;

/// Run the full application stack and assert the documented HTTP flow.
///
/// `#[ignore]` is the Rust analogue of Go's `//go:build e2e` tag: plain
/// `cargo test` compiles but does not run it. Run explicitly with
/// `cargo test --test e2e -- --include-ignored` (or `task test-e2e`).
#[ignore = "live-infra e2e; run with --include-ignored after `docker compose up -d` + `task migrate-up`"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_end_to_end() -> anyhow::Result<()> {
    let mut cfg = match Config::load() {
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
    // config.yaml enables the four features; force them here so the e2e does
    // not depend on the checkout's yaml or on `*_ENABLED` env overrides.
    common::enable_all_features(&mut cfg);

    // Bind a free port ourselves, then point the config at it. `run_with_config`
    // binds the HTTP listener via `cfg.http_addr`; binding once here lets us
    // predict the port and free it before the server re-binds.
    let probe = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let http_port = probe.local_addr().unwrap().port();
    drop(probe);
    cfg.http.host = "127.0.0.1".to_string();
    cfg.http.port = http_port;
    cfg.grpc.host = "127.0.0.1".to_string();
    cfg.grpc.port = pick_ephemeral_port().await;

    // Run the server in a background task. It blocks on a SIGTERM/SIGINT signal
    // we never send; we cancel the task to trigger shutdown.
    let cfg_for_task = cfg.clone();
    let handle = tokio::spawn(async move {
        let _ = zercle_rust_template::run_with_config(cfg_for_task).await;
    });

    let url = format!("http://127.0.0.1:{http_port}");
    wait_for_port(http_port, Duration::from_secs(10)).await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // --- Liveness -----------------------------------------------------
    let resp = client.get(format!("{url}/healthz")).send().await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK, "GET /healthz");

    // --- Readiness (poll until DB + Valkey report healthy) -----------
    let mut ready = false;
    for _ in 0..40 {
        let r = client.get(format!("{url}/readyz")).send().await?;
        if r.status() == reqwest::StatusCode::OK {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(ready, "GET /readyz never returned 200");

    // --- catalog: create a product -----------------------------------
    let resp = client
        .post(format!("{url}/api/v1/products"))
        .json(&json!({"name": "cola", "price_cents": 150, "stock": 10}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::CREATED,
        "POST /api/v1/products"
    );
    let product: Value = resp.json().await?;
    let product_id = product["id"].as_str().expect("product id").to_string();
    assert_eq!(product["name"], "cola");
    assert_eq!(product["price_cents"], 150);
    assert_eq!(product["stock"], 10);

    // --- machines: create a machine whose bank can make 50c change ----
    let resp = client
        .post(format!("{url}/api/v1/machines"))
        .json(&json!({"label": "lobby", "initial_coins": [25, 25, 50, 100]}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::CREATED,
        "POST /api/v1/machines"
    );
    let machine: Value = resp.json().await?;
    let machine_id = machine["id"].as_str().expect("machine id").to_string();
    assert_eq!(machine["label"], "lobby");
    assert_eq!(machine["coin_bank"]["50"], 1);

    // --- sales: purchase with a 50c overpay; assert the response shape -
    let resp = client
        .post(format!("{url}/api/v1/purchases"))
        .json(&json!({
            "machine_id": machine_id.clone(),
            "product_id": product_id.clone(),
            "coins": [100, 100],
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::CREATED,
        "POST /api/v1/purchases"
    );
    let purchase: Value = resp.json().await?;
    assert_eq!(purchase["machine_id"].as_str(), Some(machine_id.as_str()));
    assert_eq!(purchase["product_id"].as_str(), Some(product_id.as_str()));
    assert_eq!(purchase["price_cents"], 150);
    assert_eq!(purchase["total_inserted_cents"], 200);
    assert_eq!(purchase["change_cents"], 50);
    assert_eq!(purchase["change_coins"], json!([50]));
    assert!(
        purchase["id"].as_str().is_some_and(|s| !s.is_empty()),
        "purchase id"
    );
    assert!(
        purchase["purchased_at"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "purchased_at"
    );

    // --- reporting: summary reflects the flow -------------------------
    // `>=` assertions: the database is shared, so earlier runs may leave rows.
    // `top` is capped at the configured `reporting.max_top_machines` (20); the
    // leaderboard still lists our machine for these small local datasets.
    let resp = client
        .get(format!("{url}/api/v1/reports/summary?top=20"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "GET /api/v1/reports/summary"
    );
    let summary: Value = resp.json().await?;
    assert!(
        summary["catalog"]["product_count"].as_i64().unwrap_or(0) >= 1,
        "product_count: {summary}"
    );
    assert!(
        summary["machines"]["machine_count"].as_i64().unwrap_or(0) >= 1,
        "machine_count: {summary}"
    );
    assert!(
        summary["sales"]["purchase_count"].as_i64().unwrap_or(0) >= 1,
        "purchase_count: {summary}"
    );
    assert!(
        summary["sales"]["revenue_cents"].as_i64().unwrap_or(0) >= 150,
        "revenue_cents: {summary}"
    );
    let top = summary["top_machines"]
        .as_array()
        .expect("top_machines array");
    assert!(!top.is_empty(), "top_machines should list the sold machine");
    assert!(
        top.iter().any(|row| {
            row["label"] == "lobby"
                && row["purchase_count"].as_i64().unwrap_or(0) >= 1
                && row["revenue_cents"].as_i64().unwrap_or(0) >= 150
        }),
        "top_machines should include the lobby machine: {top:?}"
    );

    // --- error envelope: unknown product -> 404 NOT_FOUND -------------
    let unknown = uuid::Uuid::new_v4();
    let resp = client
        .get(format!("{url}/api/v1/products/{unknown}"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::NOT_FOUND,
        "GET /api/v1/products/{{unknown}}"
    );
    let body: Value = resp.json().await?;
    assert_eq!(body["error"], "NOT_FOUND");
    assert_eq!(body["message"], "resource not found");

    handle.abort();
    let _ = handle.await;
    Ok(())
}

async fn pick_ephemeral_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}

async fn wait_for_port(port: u16, timeout: Duration) -> std::io::Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("port {port} never opened"),
    ))
}
