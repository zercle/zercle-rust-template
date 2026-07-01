//! End-to-end smoke test: boot the full application (`app::run`) against
//! real Postgres + Valkey and probe every documented public route.
//!
//! Skips cleanly when the backing services are not reachable. This mirrors
//! the Go template's `TestServer_EndToEnd` — the suite is still useful on a
//! developer machine with docker-compose running.

mod common;

use std::time::Duration;

use tokio::net::TcpListener;

use zercle_rust_template::config::Config;

/// Run the full application stack and assert that the documented HTTP probes
/// all return the expected status codes.
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

    // Bind a free port ourselves, then point the config at it. `app::run`
    // binds the HTTP listener via `cfg.http_addr`; we cannot reuse the
    // already-bound listener, but binding once here lets us predict the port
    // and free it before `app::run` re-binds.
    let probe = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let http_port = probe.local_addr().unwrap().port();
    drop(probe);
    cfg.http.host = "127.0.0.1".to_string();
    cfg.http.port = http_port;
    cfg.grpc.host = "127.0.0.1".to_string();
    cfg.grpc.port = pick_ephemeral_port().await;

    // Run the server in a background task. It will block on a SIGTERM/SIGINT
    // signal which we never send; we cancel the task to trigger shutdown.
    let cfg_for_task = cfg.clone();
    let handle = tokio::spawn(async move {
        let _ = zercle_rust_template::run_with_config(cfg_for_task).await;
    });

    // Poll until the HTTP port accepts connections (the server is up).
    let url = format!("http://127.0.0.1:{http_port}");
    wait_for_port(http_port, Duration::from_secs(5)).await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // --- Liveness -----------------------------------------------------
    let resp = client.get(format!("{url}/healthz")).send().await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK, "GET /healthz");

    // --- Readiness (poll until DB + Valkey report healthy) -----------
    let mut ready = false;
    for _ in 0..20 {
        let r = client.get(format!("{url}/readyz")).send().await?;
        if r.status() == reqwest::StatusCode::OK {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(ready, "GET /readyz never returned 200");

    // --- Prometheus metrics ------------------------------------------
    let resp = client.get(format!("{url}/metrics")).send().await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK, "GET /metrics");

    // --- POST /api/v1/items → 201 -----------------------------------
    let resp = client
        .post(format!("{url}/api/v1/items"))
        .json(&serde_json::json!({"name": "stub"}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::CREATED,
        "POST /api/v1/items"
    );

    // --- GET /api/v1/items → 200 ------------------------------------
    let resp = client.get(format!("{url}/api/v1/items")).send().await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK, "GET /api/v1/items");

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
