use axum::{
    extract::Request,
    response::Response,
};
use axum::http::HeaderValue;
use uuid::Uuid;

/// Header name for request ID
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Extension key for request ID
pub const REQUEST_ID_EXTENSION: &str = "request_id";

/// Request ID middleware that adds a unique ID to each request
pub async fn request_id_middleware(mut req: Request, next: axum::middleware::Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Store request ID in extensions for use in handlers
    req.extensions_mut().insert(request_id.clone());

    // Execute the request
    let mut response = next.run(req).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    response
}

/// Get request ID from request extensions
pub fn get_request_id(req: &Request) -> Option<&str> {
    req.extensions().get::<String>().map(|s| s.as_str())
}
