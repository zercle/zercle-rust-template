use axum::{Json, response::IntoResponse};
use serde_json::json;

/// Health check handler
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "success",
        "data": {
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}
