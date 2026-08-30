//! HTTP + gRPC server orchestration. Owns axum + tonic and the ordered shutdown.
//!
//! Mirrors Go `internal/platform/server`. The server shell is feature-agnostic:
//! HTTP feature routers arrive pre-mounted (each feature nests itself under its
//! versioned prefix) and gRPC services arrive pre-registered on the router built
//! by [`grpc_server`]. [`run`] is the top-level entry point used by the
//! composition root ([`crate::app`]).

pub mod grpc_interceptor;
pub mod http;
pub mod shutdown;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::Router;
use tokio::task::JoinHandle;
use tonic::transport::Server as TonicServer;

use crate::platform::{
    config::Config,
    health::Registry as HealthRegistry,
    telemetry::{Telemetry, shutdown as shutdown_telemetry},
};

/// Layer stack applied by [`grpc_server`] on top of the plain tonic builder.
///
/// `tower::layer::util::Stack` and `Identity` are public types, so the concrete
/// tonic `Router` type is nameable — no generics are needed anywhere in the
/// wiring (`di::register` → `app::build` → `run`).
pub type GrpcStack =
    tower::layer::util::Stack<grpc_interceptor::GrpcLogRecoverLayer, tower::layer::util::Identity>;

/// A tonic router carrying every registered feature gRPC service.
pub type GrpcRouter = tonic::transport::server::Router<GrpcStack>;

/// Process-wide server state assembled by the composition root. Cloned cheaply
/// via [`Arc`] (the underlying pools and registries are already `Arc`-based).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub db: sqlx::PgPool,
    pub valkey: redis::aio::ConnectionManager,
    pub health: Arc<HealthRegistry>,
}

pub use http::build_router;
pub use shutdown::shutdown_signal;

/// Pre-configured tonic server builder: trace spans per call (OTel parity with
/// the Go template's `otelgrpc` stats handler) plus the unary logging +
/// panic-recovery interceptor. Features call `.add_service(…)` on the returned
/// builder; keep all transport-level gRPC policy here so features stay
/// transport-dumb.
pub fn grpc_server() -> TonicServer<GrpcStack> {
    TonicServer::builder()
        .layer(grpc_interceptor::GrpcLogRecoverLayer)
        .trace_fn(|req| {
            let method = req.uri().path();
            tracing::info_span!("grpc", method = %method, otel.kind = "server", otel.status_code = tracing::field::Empty)
        })
}

/// Start the HTTP + gRPC servers, wait for a shutdown signal (or a server
/// error), then run the ordered graceful shutdown.
///
/// `api` is the merged feature HTTP router (each feature nests itself under
/// its versioned prefix); `grpc` is the tonic router built from
/// [`grpc_server`] + feature services.
pub async fn run(
    state: AppState,
    telemetry: Telemetry,
    api: Router,
    grpc: GrpcRouter,
) -> Result<()> {
    // Install the Prometheus registry for the /metrics handler.
    http::install_metrics_registry(telemetry.prometheus_registry.clone());

    let cfg = state.cfg.clone();
    let state = Arc::new(state);

    // Bind listeners first so the bind errors surface synchronously (instead
    // of hiding inside a spawned task).
    let http_addr = cfg.http_addr();
    let grpc_addr = cfg.grpc_addr();

    let http_listener = tokio::net::TcpListener::bind(&http_addr)
        .await
        .with_context(|| format!("bind http {http_addr}"))?;
    let grpc_socket_addr = grpc_addr
        .parse()
        .with_context(|| format!("parse grpc address {grpc_addr}"))?;

    tracing::info!(addr = %http_addr, "http listening");
    tracing::info!(addr = %grpc_addr, "grpc listening");

    // Shared shutdown coordinator: any of (OS signal, HTTP exit, gRPC exit)
    // triggers graceful shutdown of the other server.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

    // Forward OS signals (SIGTERM/SIGINT) into the shutdown channel so a
    // process-level signal drains both servers.
    let shutdown_tx_for_signal = shutdown_tx.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx_for_signal.send(());
    });

    // --- HTTP server (axum) ----------------------------------------------
    let router = http::build_router(state.clone(), api);
    let http_state_for_shutdown = state.clone();
    let mut http_rx = shutdown_rx.clone();
    let http_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
        axum::serve(http_listener, router)
            .with_graceful_shutdown(async move {
                let _ = http_rx.changed().await;
                tracing::info!("http graceful shutdown initiated");
            })
            .await
            .context("axum serve")
    });

    // --- gRPC server (tonic) ---------------------------------------------
    //
    // `grpc` already carries every feature service plus the platform layer
    // stack (see [`grpc_server`]); here we only drive it to completion.
    let mut grpc_rx = shutdown_rx.clone();
    let grpc_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
        grpc.serve_with_shutdown(grpc_socket_addr, async move {
            let _ = grpc_rx.changed().await;
            tracing::info!("grpc graceful shutdown initiated");
        })
        .await
        .context("tonic serve")
    });

    let shutdown_timeout = cfg.shutdown_timeout();

    // Wait for either server to exit (error or normal), then signal the other
    // to drain. We await the JoinHandles by mutable reference so the unselected
    // branch is cancelled without consuming the handle, leaving it available for
    // the follow-up `.await` in the completed branch. (tokio::JoinHandle is
    // Unpin, and tokio::select! pins each branch future internally.)
    tokio::pin!(http_handle, grpc_handle);
    let (http_result, grpc_result) = tokio::select! {
        http_res = &mut http_handle => {
            let _ = shutdown_tx.send(());
            let grpc_res = match tokio::time::timeout(shutdown_timeout, &mut grpc_handle).await {
                Ok(res) => res,
                Err(_) => {
                    tracing::warn!(timeout_secs = shutdown_timeout.as_secs(), "grpc graceful shutdown timed out; forcing");
                    grpc_handle.abort();
                    Ok(Ok(()))
                }
            };
            (http_res, grpc_res)
        }
        grpc_res = &mut grpc_handle => {
            let _ = shutdown_tx.send(());
            let http_res = match tokio::time::timeout(shutdown_timeout, &mut http_handle).await {
                Ok(res) => res,
                Err(_) => {
                    tracing::warn!(timeout_secs = shutdown_timeout.as_secs(), "http graceful shutdown timed out; forcing");
                    http_handle.abort();
                    Ok(Ok(()))
                }
            };
            (http_res, grpc_res)
        }
    };

    let http_result = match http_result {
        Ok(res) => res,
        Err(join_err) => Err(anyhow::anyhow!("http server task panicked: {join_err}")),
    };
    let grpc_result = match grpc_result {
        Ok(res) => res,
        Err(join_err) => Err(anyhow::anyhow!("grpc server task panicked: {join_err}")),
    };

    if let Err(e) = &http_result {
        tracing::error!(error = %e, "http server task failed");
    }
    if let Err(e) = &grpc_result {
        tracing::error!(error = %e, "grpc server task failed");
    }

    // --- Ordered shutdown ------------------------------------------------
    shutdown(
        http_state_for_shutdown.as_ref(),
        telemetry,
        shutdown_timeout,
    )
    .await;

    // Surface whichever error is most informative.
    match (http_result, grpc_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}

/// Ordered graceful shutdown: HTTP drain (already in-flight via
/// `with_graceful_shutdown`), gRPC drain (bounded), DB close, Valkey drop,
/// telemetry flush. Bounded by `shutdown_timeout`.
async fn shutdown(state: &AppState, telemetry: Telemetry, shutdown_timeout: Duration) {
    tracing::info!(
        timeout_secs = shutdown_timeout.as_secs(),
        "shutdown initiated"
    );

    // gRPC: drain already happened in `run` via the per-task
    // `serve_with_shutdown(signal)` future — `grpc_handle.await` above blocks
    // until tonic stops accepting new requests and finishes in-flight ones
    // (functionally equivalent to Go's `GracefulStop`). There is no separate
    // tonic handle to drive `GracefulStop` / `Stop` from here, so no extra
    // step is needed in this function. The overall `shutdown_timeout` still
    // bounds the later steps (DB close, Valkey drop, telemetry flush).
    tracing::info!("grpc stopped");

    // PostgreSQL pool close.
    {
        let pool = state.db.clone();
        tokio::time::timeout(shutdown_timeout, async {
            pool.close().await;
        })
        .await
        .unwrap_or_else(|_| {
            tracing::warn!("pg pool close timed out");
        });
    }
    tracing::info!("pg pool closed");

    // Valkey: `ConnectionManager` doesn't expose an explicit close API; the
    // connection is released automatically when the main `AppState` (held via
    // `Arc` in `run`) is dropped at process exit. There is no explicit close
    // to perform here.
    tracing::info!("valkey connection released");

    // Telemetry flush + provider shutdown.
    shutdown_telemetry(telemetry);
    tracing::info!("telemetry flushed");

    tracing::info!("shutdown complete");
}
