//! Panic → 500 recovery middleware.
//!
//! Mirrors `internal/shared/middleware/recover.go` (structure.md §9). Catches panics in downstream
//! handlers, logs them with the request id (when present), and returns the shared
//! [`AppError::Internal`](crate::shared::errors::AppError::Internal) JSON body
//! `{"error":"INTERNAL","message":"internal error"}` with status 500.
//!
//! Implemented via [`tower_http::catch_panic::CatchPanicLayer`] with a custom response generator
//! so the response matches the rest of the API's error contract.

use std::any::Any;

use axum::{body::Body, http::Response, response::IntoResponse};
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

use crate::platform::errors::AppError;

/// Inner panic handler that produces the same JSON body as `AppError::Internal.into_response()`.
#[derive(Clone, Copy, Debug)]
pub struct PanicToAppError;

impl ResponseForPanic for PanicToAppError {
    type ResponseBody = Body;

    fn response_for_panic(
        &mut self,
        err: Box<dyn Any + Send + 'static>,
    ) -> Response<Self::ResponseBody> {
        // Mirror Go's logging: attach the panic payload as a `tracing::error!` field, but do not
        // leak it into the response body (Go returns the same INTERNAL payload).
        if let Some(s) = err.downcast_ref::<&'static str>() {
            tracing::error!(panic = %s, "request panic recovered");
        } else if let Some(s) = err.downcast_ref::<String>() {
            tracing::error!(panic = %s, "request panic recovered");
        } else {
            tracing::error!(panic = "<non-string>", "request panic recovered");
        }
        AppError::Internal { cause: None }.into_response()
    }
}

/// `tower::Layer` wrapper exposing the recovery middleware as a composable layer.
pub fn layer() -> CatchPanicLayer<PanicToAppError> {
    CatchPanicLayer::custom(PanicToAppError)
}

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, http::Request, routing::get};
    use tower::ServiceExt;

    use super::*;

    async fn panic_now() {
        panic!("boom");
    }

    fn app() -> Router {
        Router::new().route("/boom", get(panic_now)).layer(layer())
    }

    #[tokio::test]
    async fn panicking_handler_yields_500_internal_body() {
        let resp = app()
            .oneshot(Request::builder().uri("/boom").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 500);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(
            s.contains("\"error\":\"INTERNAL\""),
            "body should carry the INTERNAL code, got: {s}"
        );
        assert!(
            s.contains("\"message\":\"internal error\""),
            "body should carry the canonical message, got: {s}"
        );
    }

    #[tokio::test]
    async fn non_panicking_handler_passes_through() {
        async fn ok() -> &'static str {
            "ok"
        }
        let app = Router::new().route("/", get(ok)).layer(layer());
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
    }
}
