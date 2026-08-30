//! Per-request access log middleware.
//!
//! Mirrors `internal/shared/middleware/access_log.go` (structure.md §9). Emits one
//! `tracing::info!` per request with `request_id`, `method`, `path`, `status`, `latency_us`.
//!
//! The request id is read from the request extensions (populated by the upstream
//! [`crate::middleware::request_id`] middleware).

use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use tracing::info;

use crate::platform::middleware::request_id::RequestId;

/// The underlying axum middleware function. Wrap with [`axum::middleware::from_fn`] (or use
/// [`layer`]).
pub async fn middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_default();

    let response = next.run(req).await;
    let status = response.status().as_u16();
    let latency_us = start.elapsed().as_micros() as u64;

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = status,
        latency_us,
        "http request"
    );

    response
}

/// `tower::Layer` wrapper exposing the middleware as a composable layer for `.layer(layer())`.
///
/// Returns an opaque type (`impl Layer<...>`). If a named type is required, wrap manually via
/// `axum::middleware::from_fn(middleware)`.
pub fn layer() -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn::<_, ()>(middleware)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{Router, body::Body, extract::Request, http::StatusCode, routing::get};
    use tower::ServiceExt;

    use super::*;

    #[derive(Default, Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Capture {
        fn bytes(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    #[derive(Clone)]
    struct MakeWriter(Arc<Capture>);

    impl std::io::Write for MakeWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn install_capture(cap: Capture) {
        use tracing_subscriber::layer::SubscriberExt;
        let make_writer = cap.clone();
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new("info"))
            .with(tracing_subscriber::fmt::layer().json().with_writer(
                move || -> Box<dyn std::io::Write + Send> {
                    Box::new(MakeWriter(Arc::new(make_writer.clone())))
                },
            ));
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    async fn ok() -> &'static str {
        "ok"
    }

    fn app() -> Router {
        Router::new()
            .route("/", get(ok))
            .layer(axum::middleware::from_fn(middleware))
    }

    #[tokio::test]
    async fn emits_one_log_per_request_with_status_and_latency() {
        let cap = Capture::default();
        install_capture(cap.clone());

        let resp = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        tokio::task::yield_now().await;

        let bytes = cap.bytes();
        let s = String::from_utf8_lossy(&bytes);
        assert!(
            s.contains("\"http request\""),
            "expected the access-log message in captured output, got: {s}"
        );
        assert!(s.contains("\"path\":\"/\""), "path field missing: {s}");
        assert!(s.contains("\"status\":200"), "status field missing: {s}");
        assert!(
            s.contains("\"method\":\"GET\""),
            "method field missing: {s}"
        );
        assert!(s.contains("latency_us"), "latency_us field missing: {s}");
    }

    #[tokio::test]
    async fn propagates_inner_status_when_handler_returns_non_2xx() {
        async fn boom() -> StatusCode {
            StatusCode::IM_A_TEAPOT
        }
        let app = Router::new()
            .route("/boom", get(boom))
            .layer(axum::middleware::from_fn(middleware));
        let resp = app
            .oneshot(Request::builder().uri("/boom").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
    }
}
