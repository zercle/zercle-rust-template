use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use axum::extract::Request;
use axum::response::Response;

use crate::internal::domain::error::DomainError;

/// In-memory rate limiter using sliding window approach
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Check if a request should be rate limited
    pub fn check_rate_limit(&self, key: &str) -> Result<(), DomainError> {
        let now = Instant::now();
        let window_start = now - self.window;

        let mut requests = self.requests.write().unwrap();

        // Get existing requests for this key and clean up old ones
        let entry = requests.entry(key.to_string()).or_default();
        entry.retain(|&time| time > window_start);

        // Check if we're over the limit
        if entry.len() >= self.max_requests {
            return Err(DomainError::Validation(format!(
                "Rate limit exceeded. Maximum {} requests per {} seconds",
                self.max_requests,
                self.window.as_secs()
            )));
        }

        // Record this request
        entry.push(now);

        Ok(())
    }

    /// Get remaining requests for a key
    pub fn remaining_requests(&self, key: &str) -> usize {
        let now = Instant::now();
        let window_start = now - self.window;

        let requests = self.requests.read().unwrap();
        if let Some(entry) = requests.get(key) {
            let valid_count = entry.iter().filter(|&&time| time > window_start).count();
            self.max_requests.saturating_sub(valid_count)
        } else {
            self.max_requests
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    req: Request,
    next: axum::middleware::Next,
) -> Result<Response, DomainError> {
    // Use X-Forwarded-For header or client IP as the rate limit key
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    limiter.check_rate_limit(&client_ip)?;

    let response = next.run(req).await;

    // You could add rate limit headers here if needed
    Ok(response)
}
