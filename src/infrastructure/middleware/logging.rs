//! Logging middleware
//!
//! This module handles request/response logging with tracing.

use crate::config::Settings;
use axum::http::Request;
use tower_http::trace::TraceLayer;
use tracing::Level;

/// Create a TraceLayer for request/response logging
pub fn create_logging_layer(settings: &Settings) {
    let _level = match settings.logging.level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let _layer = TraceLayer::new_for_http()
        .make_span_with(move |request: &Request<()>| {
            tracing::info!(
                method = %request.method(),
                uri = %request.uri(),
                "Started processing request"
            );
        })
        .on_response(|response: &axum::http::Response<()>, latency: std::time::Duration, _span: &tracing::Span| {
            let status = response.status();
            if status.is_success() {
                tracing::info!(status = %status.as_u16(), latency = ?latency, "Completed request");
            } else if status.is_client_error() {
                tracing::warn!(status = %status.as_u16(), latency = ?latency, "Client error response");
            } else {
                tracing::error!(status = %status.as_u16(), latency = ?latency, "Server error response");
            }
        });
}

/// Create a default trace layer with INFO level
pub fn create_default_logging_layer() {
    let _layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<()>| {
            tracing::info!(method = %request.method(), uri = %request.uri(), "Started processing request");
        })
        .on_response(|response: &axum::http::Response<()>, latency: std::time::Duration, _span: &tracing::Span| {
            tracing::info!(status = %response.status().as_u16(), latency = ?latency, "Completed request");
        });
}
