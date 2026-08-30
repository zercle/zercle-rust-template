//! `X-Request-ID` propagation / generation middleware.
//!
//! Mirrors `internal/shared/middleware/request_id.go` from the Go template (structure.md §9,
//! canvas row 8). Behavior:
//!
//! * Reads `X-Request-ID` from the inbound request. Accepts it iff non-empty, ≤ 128 chars, and
//!   every char is in `[A-Za-z0-9_-]`. Otherwise generates a UUIDv7 (Go uses `uuid.NewString`;
//!   UUIDv7 is the v7-feature equivalent — see decision in canvas row 15).
//! * Stores the id in request extensions as [`RequestId`] and on the response as the same header.
//!
//! Use [`middleware`] with [`axum::middleware::from_fn`], or call [`layer`] to obtain a
//! pre-wrapped `axum::middleware::FromFnLayer` for `.layer(...)` composition.

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

/// HTTP header carrying the request id.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Hard cap on the length of an accepted client-supplied request id.
pub const MAX_REQUEST_ID_LEN: usize = 128;

/// Request extension carrying the resolved request id.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Resolve the request id for an inbound request, honoring the Go parity rules.
fn resolve_request_id(req: &Request) -> String {
    if let Some(value) = req.headers().get(&REQUEST_ID_HEADER) {
        if let Ok(s) = value.to_str() {
            if is_valid_request_id(s) {
                return s.to_owned();
            }
        }
    }
    uuid::Uuid::now_v7().to_string()
}

/// Mirrors `isValidRequestID` in Go: charset `[A-Za-z0-9_-]`, length 1..=128.
fn is_valid_request_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_REQUEST_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The underlying axum middleware function. Wrap with [`axum::middleware::from_fn`] (or use
/// [`layer`]).
pub async fn middleware(mut req: Request, next: Next) -> Response {
    let id = resolve_request_id(&req);
    let header_value = HeaderValue::from_str(&id).expect("UUIDv7 string is always valid ASCII");
    req.extensions_mut().insert(RequestId(id.clone()));
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    resp
}

/// `tower::Layer` wrapper exposing the middleware as a composable layer for `.layer(layer())`.
///
/// Returns an opaque type (`impl Layer<...>`) — this keeps the async fn's opaque future hidden
/// from the public surface. If a named type is required, wrap manually via
/// `axum::middleware::from_fn(middleware)`.
pub fn layer() -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn::<_, ()>(middleware)
}

/// Extract the request id from a request's extensions, if present.
pub fn current(req: &Request) -> Option<&str> {
    req.extensions().get::<RequestId>().map(|r| r.as_str())
}

/// Read the request id from a response's headers.
pub fn from_response(resp: &Response) -> Option<&str> {
    resp.headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, extract::Request, routing::get};
    use tower::ServiceExt;

    async fn ok() -> &'static str {
        "ok"
    }

    fn app() -> Router {
        Router::new()
            .route("/", get(ok))
            .layer(axum::middleware::from_fn(middleware))
    }

    #[tokio::test]
    async fn generates_when_missing() {
        let resp = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let id = from_response(&resp).expect("response header present");
        assert!(!id.is_empty());
        assert!(id.len() <= MAX_REQUEST_ID_LEN);
        assert_eq!(id.len(), 36, "generated id should be a UUIDv7 string: {id}");
    }

    #[tokio::test]
    async fn rejects_invalid_charset_and_regenerates() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "has space")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = from_response(&resp).unwrap();
        assert!(!id.contains(' '));
        assert_eq!(id.len(), 36, "should fall back to UUIDv7: {id}");
    }

    #[tokio::test]
    async fn rejects_slash_and_regenerates() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "abc/def")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = from_response(&resp).unwrap();
        assert!(!id.contains('/'));
    }

    #[tokio::test]
    async fn accepts_valid_charset() {
        let supplied = "Abc-123_DEF_xYz-09";
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, supplied)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(from_response(&resp), Some(supplied));
    }

    #[tokio::test]
    async fn rejects_overlong_and_regenerates() {
        let too_long = "a".repeat(MAX_REQUEST_ID_LEN + 1);
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, too_long.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = from_response(&resp).unwrap();
        assert!(id.len() <= MAX_REQUEST_ID_LEN);
        assert_eq!(id.len(), 36);
    }

    #[test]
    fn validator_unit_table() {
        assert!(is_valid_request_id("a"));
        assert!(is_valid_request_id("Z-0_9"));
        assert!(is_valid_request_id(&"a".repeat(MAX_REQUEST_ID_LEN)));
        assert!(!is_valid_request_id(""));
        assert!(!is_valid_request_id(" "));
        assert!(!is_valid_request_id("a/b"));
        assert!(!is_valid_request_id("a.b"));
        assert!(!is_valid_request_id(&"a".repeat(MAX_REQUEST_ID_LEN + 1)));
        assert!(!is_valid_request_id("a\nb"));
    }

    #[tokio::test]
    async fn extension_is_available_to_handlers() {
        async fn echo_id(req: Request) -> String {
            current(&req).unwrap_or_default().to_owned()
        }
        let app = Router::new()
            .route("/", get(echo_id))
            .layer(axum::middleware::from_fn(middleware));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "supplied-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"supplied-1");
    }
}
