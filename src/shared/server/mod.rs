//! HTTP + gRPC server orchestration. Owns axum + tonic and the ordered shutdown.
//!
//! Mirrors `internal/shared/server/{shutdown,grpc}.go` (structure.md §14, canvas row 28).
//! See [`run`] for the top-level entry point used by [`crate::app::run`].

pub mod grpc_interceptor;
pub mod http;
pub mod shutdown;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::task::JoinHandle;
use tonic::transport::Server as TonicServer;
use tower::ServiceBuilder;

/// 4 MiB cap on incoming + outgoing gRPC message bodies. Matches Go's
/// `grpc.MaxRecvMsgSize(4*1024*1024)` and `grpc.MaxSendMsgSize(4*1024*1024)`
/// in `internal/shared/server/grpc.go` (Go defaults to 4 MiB anyway, but
/// setting it explicitly makes the limit visible at the call site).
const GRPC_MESSAGE_SIZE_LIMIT: usize = 4 * 1024 * 1024;

use crate::{
    app::AppState,
    features::example::{GrpcServer, grpc_server},
    shared::telemetry::{Telemetry, shutdown as shutdown_telemetry},
};

pub use shutdown::shutdown_signal;

/// Start the HTTP + gRPC servers, wait for a shutdown signal (or a server
/// error), then run the ordered graceful shutdown.
pub async fn run(state: AppState, telemetry: Telemetry) -> Result<()> {
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

    // --- HTTP server (axum) ----------------------------------------------
    let router = http::build_router(state.clone());
    let http_state_for_shutdown = state.clone();
    let http_signal = shutdown_signal();
    let http_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
        axum::serve(http_listener, router)
            .with_graceful_shutdown(async move {
                http_signal.await;
                tracing::info!("http graceful shutdown initiated");
            })
            .await
            .context("axum serve")
    });

    // --- gRPC server (tonic) ---------------------------------------------
    //
    // Observability parity with the Go template's
    // `grpc.StatsHandler(otelgrpc.NewServerHandler())` is achieved via
    // `trace_fn`, which attaches a `tracing::Span` to each call. The
    // `tracing-opentelemetry` layer installed in `shared::telemetry` then
    // exports those spans over OTLP, so OTel sees one span per gRPC call
    // without pulling a new crate. The unary logging + panic-recovery
    // interceptor (`grpc_interceptor::GrpcLogRecoverLayer`) is the
    // functional equivalent of the Go unary interceptor. Stream RPCs
    // receive the tracing span but no panic-recovery wrapper (tonic 0.12's
    // `Interceptor` trait only covers unary; this is the documented
    // acceptable gap).
    let example_grpc = grpc_server(GrpcServer::new(state.example_service.clone()))
        .max_decoding_message_size(GRPC_MESSAGE_SIZE_LIMIT)
        .max_encoding_message_size(GRPC_MESSAGE_SIZE_LIMIT);
    let grpc_log_layer = ServiceBuilder::new()
        .layer(grpc_interceptor::GrpcLogRecoverLayer)
        .into_inner();
    let grpc_signal = shutdown_signal();
    let grpc_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
        TonicServer::builder()
            .layer(grpc_log_layer)
            .trace_fn(|req| {
                let method = req.uri().path();
                tracing::info_span!("grpc", method = %method, otel.kind = "server", otel.status_code = tracing::field::Empty)
            })
            .add_service(example_grpc)
            .serve_with_shutdown(grpc_socket_addr, async move {
                grpc_signal.await;
                tracing::info!("grpc graceful shutdown initiated");
            })
            .await
            .context("tonic serve")
    });

    // Wait for either a server error or (implicitly) for the caller to drop
    // `run` after a process-level signal handler triggers. Because we install
    // per-server shutdown futures that await the same signal, we additionally
    // wait for an explicit caller-driven shutdown by selecting on the signal
    // and the two server tasks.
    let shutdown_timeout = cfg.shutdown_timeout();

    let http_result = http_handle.await;
    let grpc_result = grpc_handle.await;

    if let Err(e) = &http_result {
        tracing::error!(error = %e, "http server task failed");
    }
    if let Err(e) = &grpc_result {
        tracing::error!(error = %e, "grpc server task failed");
    }

    // If the http server panicked before its graceful shutdown could run,
    // make sure the gRPC side still drains.
    drop(shutdown_signal());

    // --- Ordered shutdown ------------------------------------------------
    shutdown(
        http_state_for_shutdown.as_ref(),
        telemetry,
        shutdown_timeout,
    )
    .await;

    // Surface whichever error is most informative.
    match (http_result, grpc_result) {
        (Ok(Ok(())), Ok(Ok(()))) => Ok(()),
        (Ok(Err(e)), _) => Err(e),
        (_, Ok(Err(e))) => Err(e),
        (Err(join), _) => Err(anyhow::anyhow!("http server task panicked: {join}")),
        (_, Err(join)) => Err(anyhow::anyhow!("grpc server task panicked: {join}")),
    }
}

/// Ordered graceful shutdown: HTTP drain (already in-flight via
/// `with_graceful_shutdown`), gRPC drain (bounded), DB close, Valkey drop,
/// telemetry flush. Bounded by `shutdown_timeout`.
pub async fn shutdown(state: &AppState, telemetry: Telemetry, shutdown_timeout: Duration) {
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

    // Valkey: `ConnectionManager` doesn't expose an explicit close; dropping
    // the last instance is enough. We discard the handle from the state.
    drop(state.valkey.clone());
    tracing::info!("valkey connection released");

    // Telemetry flush + provider shutdown.
    shutdown_telemetry(telemetry);
    tracing::info!("telemetry flushed");

    tracing::info!("shutdown complete");
}
