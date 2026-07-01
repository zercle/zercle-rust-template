//! gRPC unary tower middleware: panic recovery + per-call logging.
//!
//! Mirrors the unary interceptor in `internal/shared/server/grpc.go` of the
//! Go template (which also installs an OTel `StatsHandler` and a
//! `MaxRecvMsgSize` / `MaxSendMsgSize` limit; the message-size limits are
//! applied at the per-service builder in
//! [`crate::shared::server::run`], and OTel tracing is achieved via
//! `Server::trace_fn` which the existing `tracing-opentelemetry` layer picks
//! up — together matching Go's behavior without pulling a new crate).
//!
//! What this middleware does on every unary call:
//! 1. Records the start instant and extracts the gRPC method from
//!    `Request::uri().path()` (e.g. `/example.v1.ExampleService/CreateItem`).
//! 2. Wraps the inner service future with `catch_unwind` so a panic in the
//!    handler is recovered, logged via `tracing::error!`, and converted into a
//!    `tonic::Status::internal("internal error")` response — mirroring Go's
//!    `recoverGRPCPanic` helper that returns `codes.Internal`.
//! 3. On normal completion, logs the call's latency and the resulting
//!    `grpc-status` header at INFO for success / WARN for non-zero status.
//!
//! Note on streams: tonic 0.12's `Interceptor` trait operates on `Request<()>`
//! (metadata only) and cannot wrap a streaming body, so this middleware is
//! unary-only. Streams still receive a `tracing` span via `Server::trace_fn`
//! for OTel parity; panic recovery on streams is the documented acceptable
//! gap (see `shared/server/mod.rs`).

use std::{
    any::Any,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use futures::{FutureExt, future::BoxFuture};
use http::{HeaderMap, Request, Response};
use tonic::{Status, body::BoxBody};
use tower::{Layer, Service};

const GRPC_STATUS_HEADER: &str = "grpc-status";
type GrpcBody = BoxBody;
type GrpcError = Box<dyn std::error::Error + Send + Sync>;

/// Tower layer that wraps a tonic gRPC service. See module docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct GrpcLogRecoverLayer;

impl<S> Layer<S> for GrpcLogRecoverLayer {
    type Service = GrpcLogRecoverService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcLogRecoverService { inner }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcLogRecoverService<S> {
    inner: S,
}

impl<S, E> Service<Request<GrpcBody>> for GrpcLogRecoverService<S>
where
    S: Service<Request<GrpcBody>, Response = Response<GrpcBody>, Error = E>
        + Clone
        + Send
        + 'static,
    E: Into<GrpcError> + Send + std::fmt::Display + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<GrpcBody>;
    type Error = E;
    type Future = GrpcLogRecoverFuture<E>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<GrpcBody>) -> Self::Future {
        let method = req.uri().path().to_owned();
        let start = Instant::now();
        let mut inner = self.inner.clone();
        // `AssertUnwindSafe` is sound here: any panic in the handler has
        // already corrupted application state, but tonic asks us to swallow
        // it and surface an Internal status (Go does the same); we never
        // re-enter the inner service with the same captured state.
        let caught = AssertUnwindSafe(async move { inner.call(req).await })
            .catch_unwind()
            .boxed();
        GrpcLogRecoverFuture {
            inner: Some(caught),
            method,
            start,
            _phantom: std::marker::PhantomData,
        }
    }
}

type CaughtResult<E> = Result<Result<Response<GrpcBody>, E>, Box<dyn Any + Send + 'static>>;

/// Future produced by [`GrpcLogRecoverService`]. Holds the
/// `catch_unwind`-wrapped inner service future plus the metadata needed to
/// produce the completion log line.
pub struct GrpcLogRecoverFuture<E> {
    inner: Option<BoxFuture<'static, CaughtResult<E>>>,
    method: String,
    start: Instant,
    _phantom: std::marker::PhantomData<fn() -> E>,
}

impl<E> Future for GrpcLogRecoverFuture<E>
where
    E: Into<GrpcError> + std::fmt::Display,
{
    type Output = Result<Response<GrpcBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: all fields are `Unpin` (BoxFuture is Unpin, String is
        // Unpin, Instant is Unpin, PhantomData is Unpin), so `get_mut` is
        // sound.
        let this = self.get_mut();
        let fut = this
            .inner
            .as_mut()
            .expect("grpc interceptor future polled after completion");
        match fut.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(Ok(response))) => {
                log_completion(&this.method, this.start, &response);
                this.inner = None;
                Poll::Ready(Ok(response))
            }
            Poll::Ready(Ok(Err(err))) => {
                // Transport/infrastructure-level error from the inner tower service.
                // NOTE: tonic encodes application gRPC statuses (NotFound,
                // InvalidArgument, PermissionDenied, ...) in the *response
                // headers/trailers*, so they are surfaced as `Ok(Response)` and
                // logged by `log_completion` above — at `warn` for non-zero
                // `grpc-status`. This `Err` arm therefore only fires for genuine
                // transport failures (e.g. connection errors), which warrant
                // `error` level. Do not lower this to `warn`.
                let latency_us = this.start.elapsed().as_micros() as u64;
                tracing::error!(
                    method = %this.method,
                    latency_us,
                    error = %err,
                    "grpc request failed"
                );
                this.inner = None;
                Poll::Ready(Err(err))
            }
            Poll::Ready(Err(panic_payload)) => {
                log_panic(&this.method, &panic_payload);
                this.inner = None;
                Poll::Ready(Ok(Status::internal("internal error").into_http()))
            }
        }
    }
}

fn log_completion(method: &str, start: Instant, response: &Response<GrpcBody>) {
    let latency_us = start.elapsed().as_micros() as u64;
    let status = grpc_status_code(response.headers());
    if status == 0 {
        tracing::info!(
            method = %method,
            latency_us,
            grpc_status = status,
            "grpc request completed"
        );
    } else {
        tracing::warn!(
            method = %method,
            latency_us,
            grpc_status = status,
            "grpc request completed"
        );
    }
}

fn log_panic(method: &str, payload: &Box<dyn Any + Send + 'static>) {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        tracing::error!(
            method = %method,
            panic = %s,
            "grpc unary panic recovered"
        );
    } else if let Some(s) = payload.downcast_ref::<String>() {
        tracing::error!(
            method = %method,
            panic = %s,
            "grpc unary panic recovered"
        );
    } else {
        tracing::error!(
            method = %method,
            panic = "<non-string>",
            "grpc unary panic recovered"
        );
    }
}

/// Extract the gRPC status code from a response. Returns 0 (OK) when the
/// trailer is missing — tonic always sets it, but a missing header should
/// not be treated as an error.
fn grpc_status_code(headers: &HeaderMap) -> i32 {
    extract_status(headers).unwrap_or(0)
}

fn extract_status(headers: &HeaderMap) -> Option<i32> {
    headers
        .get(GRPC_STATUS_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i32>().ok())
}

// --- thin AssertUnwindSafe wrapper to keep the import tidy ------------

#[derive(Debug)]
struct AssertUnwindSafe<T>(T);

impl<T> std::panic::UnwindSafe for AssertUnwindSafe<T> {}
impl<T> std::panic::RefUnwindSafe for AssertUnwindSafe<T> {}

impl<T: Future> Future for AssertUnwindSafe<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: AssertUnwindSafe<T> is a transparent wrapper; projection
        // through `Pin<&mut Self>` into the inner T is the standard
        // pin-projection pattern.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        task::{Context, Poll},
    };

    use http::{HeaderValue, Request, Response};
    use tower::ServiceExt;

    use super::*;

    /// Test inner that just succeeds with grpc-status 0 and an empty body.
    #[derive(Clone)]
    struct OkService;

    impl Service<Request<GrpcBody>> for OkService {
        type Response = Response<GrpcBody>;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Response<GrpcBody>, Infallible>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<GrpcBody>) -> Self::Future {
            let resp = Response::builder()
                .header(GRPC_STATUS_HEADER, "0")
                .body(tonic::body::empty_body())
                .expect("build response");
            std::future::ready(Ok(resp))
        }
    }

    /// Test inner that panics inside the async call.
    #[derive(Clone)]
    struct PanicService;

    impl Service<Request<GrpcBody>> for PanicService {
        type Response = Response<GrpcBody>;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Response<GrpcBody>, Infallible>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<GrpcBody>) -> Self::Future {
            panic!("boom")
        }
    }

    /// Test inner that returns a non-zero gRPC status (e.g. NOT_FOUND).
    #[derive(Clone)]
    struct NotFoundService;

    impl Service<Request<GrpcBody>> for NotFoundService {
        type Response = Response<GrpcBody>;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Response<GrpcBody>, Infallible>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<GrpcBody>) -> Self::Future {
            let resp = Response::builder()
                .header(GRPC_STATUS_HEADER, "5")
                .body(tonic::body::empty_body())
                .expect("build response");
            std::future::ready(Ok(resp))
        }
    }

    fn empty_request(path: &str) -> Request<GrpcBody> {
        Request::builder()
            .uri(path)
            .header("content-type", "application/grpc")
            .body(tonic::body::empty_body())
            .expect("build request")
    }

    #[tokio::test]
    async fn success_passes_through_with_status_zero() {
        let svc = GrpcLogRecoverLayer.layer(OkService);
        let resp = svc.oneshot(empty_request("/svc/Method")).await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let status = resp
            .headers()
            .get(GRPC_STATUS_HEADER)
            .and_then(|v| v.to_str().ok());
        assert_eq!(status, Some("0"));
    }

    #[tokio::test]
    async fn panic_is_recovered_and_returned_as_internal_status() {
        let svc = GrpcLogRecoverLayer.layer(PanicService);
        let resp = svc
            .oneshot(empty_request("/svc/PanicMethod"))
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let status = resp
            .headers()
            .get(GRPC_STATUS_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i32>().ok())
            .expect("grpc-status header present");
        // Code 13 == INTERNAL — matches Go's `codes.Internal` returned from
        // `recoverGRPCPanic`. The grpc-message trailer is percent-encoded by
        // tonic (an internal detail), so we only assert on the status code
        // here; the on-the-wire message is `"internal error"`, matching
        // Go's `sharederrors.ErrInternal`.
        assert_eq!(status, 13);
    }

    #[tokio::test]
    async fn non_zero_status_passes_through_unchanged() {
        let svc = GrpcLogRecoverLayer.layer(NotFoundService);
        let resp = svc
            .oneshot(empty_request("/svc/NotFoundMethod"))
            .await
            .unwrap();
        let status = resp
            .headers()
            .get(GRPC_STATUS_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(status, "5");
    }

    #[test]
    fn extract_status_handles_missing_header_as_none() {
        let mut h = HeaderMap::new();
        assert_eq!(extract_status(&h), None);
        h.insert(GRPC_STATUS_HEADER, HeaderValue::from_static("13"));
        assert_eq!(extract_status(&h), Some(13));
    }
}
