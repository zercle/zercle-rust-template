//! axum HTTP server builder: middleware stack, shared routes, and feature mount.
//!
//! Mirrors `internal/shared/server/http.go` (structure.md §14, canvas row 26).
//!
//! Middleware order (applied as the outermost layers in the same sequence):
//! Recover → RequestID → OTel(`TraceLayer`) → AccessLog → CORS → BodyLimit.
//!
//! Shared routes (no prefix):
//! - `GET /healthz`  — liveness; 200 empty / 500 on registry error
//! - `GET /readyz`   — readiness; 200 empty / 503 `{"status":"not ready"}`
//! - `GET /metrics`  — Prometheus text exposition (Prometheus 0.0.4 content type)

use std::{str::FromStr, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    middleware::from_fn,
    response::{IntoResponse, Response},
    routing::get,
};
use prometheus::Registry;
use tower::ServiceBuilder;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::{
    features::example,
    middleware::{access_log, cors, recover, request_id},
    shared::telemetry::metrics_body as render_metrics,
};

const METRICS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";
const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Build the application router with the full middleware stack and shared routes.
///
/// The returned `Router` is `Send + 'static` and ready for `axum::serve(listener, router)`.
pub fn build_router(state: Arc<crate::app::AppState>) -> Router {
    let cfg = state.cfg.clone();

    let body_limit_bytes = parse_body_limit_bytes(&cfg.http.body_limit);
    let probe_timeout = if cfg.http.health_probe_timeout.is_empty() {
        DEFAULT_PROBE_TIMEOUT
    } else {
        cfg.http_health_probe_timeout()
    };

    let cors_layer = if cfg.http.cors_allow_origins.is_empty()
        && cfg.http.cors_allow_methods.is_empty()
        && cfg.http.cors_allow_headers.is_empty()
    {
        cors::default_layer()
    } else {
        cors::layer(&cfg)
    };

    // Outer middleware stack — order matters. ServiceBuilder applies layers in
    // declaration order (so Recover wraps RequestID wraps TraceLayer wraps ...
    // wraps BodyLimit), matching Go's `e.Use(...)` chain.
    let middleware_stack = ServiceBuilder::new()
        .layer(recover::layer())
        .layer(from_fn(request_id::middleware))
        .layer(TraceLayer::new_for_http())
        .layer(from_fn(access_log::middleware))
        .layer(cors_layer);

    let body_limit_layer = if body_limit_bytes > 0 {
        Some(RequestBodyLimitLayer::new(body_limit_bytes))
    } else {
        None
    };

    // Shared routes — use idiomatic Axum `State` extraction so the parent
    // router stays state-less and can `nest` the example feature router
    // (whose own state type is unrelated to ours). State is a tuple of
    // `Arc<AppState>` plus the probe timeout; handlers destructure it via
    // the `State` extractor.
    let shared = Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/readyz", get(readyz_handler))
        .route("/metrics", get(metrics_handler))
        .with_state((state.clone(), probe_timeout));

    // Mount the example feature under `/api/v1`.
    let app_router = shared.nest(
        "/api/v1",
        example::http_routes(state.example_service.clone()),
    );

    let app_router = app_router.layer(middleware_stack);

    if let Some(limit) = body_limit_layer {
        app_router.layer(limit)
    } else {
        app_router
    }
}

async fn healthz_handler(
    State((state, probe_timeout)): State<(Arc<crate::app::AppState>, Duration)>,
) -> Response {
    let registry = state.health.clone();
    let result = tokio::time::timeout(probe_timeout, registry.live()).await;
    match result {
        Ok(Ok(())) => StatusCode::OK.into_response(),
        Ok(Err(err)) => {
            tracing::error!(error = %err, "liveness check failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(_) => {
            tracing::error!("liveness check timed out");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn readyz_handler(
    State((state, probe_timeout)): State<(Arc<crate::app::AppState>, Duration)>,
) -> Response {
    let registry = state.health.clone();
    let result = tokio::time::timeout(probe_timeout, registry.ready()).await;
    match result {
        Ok(Ok(())) => StatusCode::OK.into_response(),
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "readiness check failed");
            let body = axum::Json(serde_json::json!({ "status": "not ready" }));
            (StatusCode::SERVICE_UNAVAILABLE, body).into_response()
        }
        Err(_) => {
            tracing::warn!("readiness check timed out");
            let body = axum::Json(serde_json::json!({ "status": "not ready" }));
            (StatusCode::SERVICE_UNAVAILABLE, body).into_response()
        }
    }
}

async fn metrics_handler(
    State((_state, _probe_timeout)): State<(Arc<crate::app::AppState>, Duration)>,
) -> Response {
    let registry = metrics_registry();
    let body = render_metrics(&registry);
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static(METRICS_CONTENT_TYPE),
        )
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ------------------------------------------------------------------
// metrics_registry — module-level slot for the Prometheus registry.
// Populated by `shared::server::run` before starting servers. This keeps
// `Telemetry` ownership clean (Telemetry is consumed for shutdown) while
// letting the HTTP /metrics handler reach the registry without an Arc in
// AppState. See `shared::server::mod.rs::run`.
// ------------------------------------------------------------------

use once_cell::sync::OnceCell;

static METRICS_REGISTRY: OnceCell<Registry> = OnceCell::new();

pub(crate) fn install_metrics_registry(registry: Registry) {
    let _ = METRICS_REGISTRY.set(registry);
}

fn metrics_registry() -> Registry {
    METRICS_REGISTRY
        .get()
        .cloned()
        .expect("metrics registry must be installed before serving /metrics")
}

/// Parse a body-limit string like `"1M"`, `"512K"`, `"2G"`, or a raw byte
/// count `"1048576"` into bytes. Returns 0 (skip the layer) on empty /
/// unparseable input. Mirrors the Go `parseBodyLimitBytes`.
pub fn parse_body_limit_bytes(s: &str) -> usize {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let upper = s.to_ascii_uppercase();
    let (suffix, multiplier): (&str, usize) = if let Some(stripped) = upper.strip_suffix('K') {
        (stripped.trim(), 1024)
    } else if let Some(stripped) = upper.strip_suffix('M') {
        (stripped.trim(), 1024 * 1024)
    } else if let Some(stripped) = upper.strip_suffix('G') {
        (stripped.trim(), 1024 * 1024 * 1024)
    } else {
        (upper.trim(), 1)
    };

    match usize::from_str(suffix) {
        Ok(n) if n > 0 => n.saturating_mul(multiplier),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_body_limit_bytes_known_units() {
        assert_eq!(parse_body_limit_bytes("1M"), 1024 * 1024);
        assert_eq!(parse_body_limit_bytes("512K"), 512 * 1024);
        assert_eq!(parse_body_limit_bytes("2G"), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_body_limit_bytes("1024"), 1024);
        assert_eq!(parse_body_limit_bytes("0"), 0);
        assert_eq!(parse_body_limit_bytes(""), 0);
        assert_eq!(parse_body_limit_bytes("garbage"), 0);
    }

    #[test]
    fn parse_body_limit_bytes_lowercase() {
        assert_eq!(parse_body_limit_bytes("1m"), 1024 * 1024);
        assert_eq!(parse_body_limit_bytes("2k"), 2 * 1024);
    }

    #[test]
    fn parse_body_limit_bytes_negative_returns_zero() {
        assert_eq!(parse_body_limit_bytes("-1"), 0);
    }
}
