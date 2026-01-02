//! Rate limiting middleware
//!
//! This module handles request rate limiting using an in-memory store.

use crate::config::Settings;
use axum::http::{header, StatusCode};
use axum::{body::Body, extract::Request, response::Response};
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_minute: u64,
    pub window_secs: u64,
}

impl From<&Settings> for RateLimitConfig {
    fn from(settings: &Settings) -> Self {
        Self {
            requests_per_minute: settings.rate_limit.requests_per_minute as u64,
            window_secs: 60,
        }
    }
}

/// Rate limit entry for tracking requests
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u64,
    window_start: Instant,
}

/// In-memory rate limiter store
#[derive(Debug, Default)]
pub struct InMemoryRateLimiter {
    entries: RwLock<std::collections::HashMap<String, RateLimitEntry>>,
}

impl InMemoryRateLimiter {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub async fn check_rate_limit(
        &self,
        key: &str,
        max_requests: u64,
        window_secs: u64,
    ) -> (bool, u64, u64) {
        let now = Instant::now();
        let window_duration = Duration::from_secs(window_secs);

        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(key) {
            if now.duration_since(entry.window_start) > window_duration {
                entry.count = 1;
                entry.window_start = now;
                return (true, max_requests - 1, window_secs);
            }

            if entry.count < max_requests {
                entry.count += 1;
                let remaining = max_requests - entry.count;
                let reset_time =
                    window_secs.saturating_sub(now.duration_since(entry.window_start).as_secs());
                return (true, remaining, reset_time);
            }

            let reset_time =
                window_secs.saturating_sub(now.duration_since(entry.window_start).as_secs());
            return (false, 0, reset_time);
        }

        entries.insert(
            key.to_string(),
            RateLimitEntry {
                count: 1,
                window_start: now,
            },
        );
        (true, max_requests - 1, window_secs)
    }
}

/// Shared rate limiter state
#[derive(Clone)]
pub struct RateLimitState {
    limiter: Arc<InMemoryRateLimiter>,
    config: RateLimitConfig,
}

impl RateLimitState {
    pub fn new(settings: &Settings) -> Self {
        Self {
            limiter: Arc::new(InMemoryRateLimiter::new()),
            config: RateLimitConfig::from(settings),
        }
    }
}

/// Error response for rate limiting
#[derive(Debug, Serialize)]
pub struct RateLimitErrorResponse {
    pub success: bool,
    pub error: String,
    pub retry_after: u64,
}

/// Rate limit middleware layer
#[derive(Clone)]
pub struct RateLimitLayer {
    state: RateLimitState,
}

impl RateLimitLayer {
    pub fn new(settings: &Settings) -> Self {
        Self {
            state: RateLimitState::new(settings),
        }
    }

    pub fn state(&self) -> &RateLimitState {
        &self.state
    }

    fn extract_client_ip(req: &Request<Body>) -> Option<String> {
        let forwarded = req.headers().get("X-Forwarded-For");
        if let Some(Ok(forwarded_str)) = forwarded.map(|h| h.to_str()) {
            let ip = forwarded_str.split(',').next()?;
            return Some(ip.trim().to_string());
        }

        let real_ip = req.headers().get("X-Real-IP");
        if let Some(Ok(ip)) = real_ip.map(|h| h.to_str()) {
            return Some(ip.to_string());
        }

        None
    }
}

pub async fn rate_limit_middleware(
    state: RateLimitState,
    mut req: Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let client_ip =
        RateLimitLayer::extract_client_ip(&req).unwrap_or_else(|| "unknown".to_string());

    let (allowed, remaining, retry_after) = state
        .limiter
        .check_rate_limit(
            &client_ip,
            state.config.requests_per_minute,
            state.config.window_secs,
        )
        .await;

    if !allowed {
        let body = r#"{"success":false,"error":"Rate limit exceeded. Please try again later.","retry_after":"# 
            .to_string()
            + &retry_after.to_string()
            + r#""}"#;

        let response = axum::http::Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::RETRY_AFTER, retry_after.to_string())
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.into())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok(response);
    }

    // Add rate limit headers to request
    if let Ok(limit_val) =
        header::HeaderValue::from_str(&state.config.requests_per_minute.to_string())
    {
        req.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-limit"),
            limit_val,
        );
    }
    if let Ok(remaining_val) = header::HeaderValue::from_str(&remaining.to_string()) {
        req.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-remaining"),
            remaining_val,
        );
    }
    if let Ok(reset_val) = header::HeaderValue::from_str(&retry_after.to_string()) {
        req.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-reset"),
            reset_val,
        );
    }

    Ok(next.run(req).await)
}
