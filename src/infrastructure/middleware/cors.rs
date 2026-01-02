//! CORS middleware
//!
//! This module handles Cross-Origin Resource Sharing configuration.

use crate::config::Settings;
use axum::http::{header, Method};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

/// Create a CORS layer from settings
pub fn create_cors_layer(settings: &Settings) -> CorsLayer {
    let allowed_origins: Vec<String> = settings.cors.allowed_origins.clone();

    // Build CORS layer
    let mut cors_layer = CorsLayer::new()
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::ACCESS_CONTROL_REQUEST_METHOD,
            header::ACCESS_CONTROL_REQUEST_HEADERS,
        ]))
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600));

    // Set allowed origins
    if allowed_origins.iter().any(|o| o == "*") {
        cors_layer = cors_layer.allow_origin(AllowOrigin::any());
    } else {
        let origins: Vec<axum::http::HeaderValue> = allowed_origins
            .iter()
            .filter(|o| !o.is_empty())
            .filter_map(|o| o.parse().ok())
            .collect();
        if !origins.is_empty() {
            cors_layer = cors_layer.allow_origin(AllowOrigin::list(origins));
        }
    }

    cors_layer
}

/// Create a permissive CORS layer for development
pub fn create_permissive_cors_layer() -> CorsLayer {
    CorsLayer::permissive()
}
