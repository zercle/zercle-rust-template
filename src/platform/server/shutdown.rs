//! Ordered graceful shutdown helpers and OS-signal handling.
//!
//! Mirrors `internal/shared/server/shutdown.go` (structure.md §14, canvas row 28).
//!
//! Sequence:
//!   1. axum HTTP graceful drain (handled by `axum::serve(...).with_graceful_shutdown`).
//!   2. tonic gRPC graceful stop (with timeout fallback `Stop`).
//!   3. PostgreSQL pool close.
//!   4. Valkey connection drop (best-effort; `ConnectionManager::new` clones).
//!   5. Telemetry flush (tracer + meter).
//!
//! The whole sequence is bounded by `cfg.app.shutdown_timeout`.

use std::time::Duration;

use anyhow::Result;

/// Wait for SIGTERM or SIGINT. Returns once either signal is received.
///
/// On non-unix platforms this falls back to awaiting `tokio::signal::ctrl_c`
/// (which is what `tokio::signal::unix::signal` would never produce). Kept
/// hidden behind cfg so `cargo check` doesn't complain about unused imports
/// on platforms where the unix module is unavailable.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler; falling back to ctrl_c");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGINT handler; falling back to sigterm");
                let _ = sigterm.recv().await;
                return;
            }
        };
        tokio::select! {
            _ = sigterm.recv() => tracing::info!("SIGTERM received"),
            _ = sigint.recv()  => tracing::info!("SIGINT received"),
        }
    }
    #[cfg(not(unix))]
    {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to await ctrl_c");
        } else {
            tracing::info!("ctrl_c received");
        }
    }
}

/// Race a graceful shutdown against a deadline; if the timeout elapses first,
/// call `fallback`. Both closures run on the current task.
pub async fn with_shutdown_deadline<F, G>(timeout: Duration, graceful: F, fallback: G) -> Result<()>
where
    F: std::future::Future<Output = Result<()>>,
    G: std::future::Future<Output = ()>,
{
    match tokio::time::timeout(timeout, graceful).await {
        Ok(res) => res,
        Err(_) => {
            tracing::warn!(
                timeout_secs = timeout.as_secs(),
                "graceful shutdown timed out; forcing"
            );
            fallback.await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn with_shutdown_deadline_returns_graceful_result_on_success() {
        let r = with_shutdown_deadline(Duration::from_secs(1), async { Ok(()) }, async {}).await;
        assert!(r.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn with_shutdown_deadline_runs_fallback_on_timeout() {
        // Graceful never resolves → deadline elapses → fallback runs → Ok.
        let r = with_shutdown_deadline(
            Duration::from_millis(10),
            async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(())
            },
            async {},
        )
        .await;
        assert!(r.is_ok());
    }
}
