//! axum HTTP handler for reporting (Go
//! `handler/handler.go` parity, port-spec §5.4a/§5.4h).
//!
//! Route (nested under `/api/v1` by the feature's `di`):
//!   `GET /reports/summary` → 200 + `SummaryResponse`  (query: `top`)
//!
//! The handler binds the feature's contract type directly, validates, and
//! calls the usecase service — it never maps to or from domain
//! entities. Generic over `S: Service` so tests inject a `MockService`.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use validator::Validate;

use crate::features::reporting::contract::{SummaryRequest, SummaryResponse};
use crate::features::reporting::usecase::Service;
use crate::platform::errors::AppError;

/// Handler holds the service as `Arc<S>`; the generic keeps the test seam clean.
pub struct Handler<S: Service + ?Sized> {
    service: Arc<S>,
}

// Manual Clone impl: `Arc<S>` is `Clone` for any `S`, including `?Sized`.
impl<S: Service + ?Sized> Clone for Handler<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
        }
    }
}

impl<S: Service + ?Sized> Handler<S> {
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

/// Build the axum router for reporting. The caller (the feature's `di`) nests
/// this under `/api/v1`.
pub fn routes<S>(service: Arc<S>) -> Router
where
    S: Service + ?Sized + Send + Sync + 'static,
{
    let state = Handler::new(service);
    Router::new()
        .route("/reports/summary", get(summary::<S>))
        .with_state(state)
}

async fn summary<S>(
    State(h): State<Handler<S>>,
    Query(req): Query<SummaryRequest>,
) -> Result<Json<SummaryResponse>, AppError>
where
    S: Service + ?Sized,
{
    req.validate().map_err(AppError::from_validation)?;
    let resp = h.service.summary(req).await.map_err(AppError::from)?;
    Ok(Json(resp))
}

#[cfg(test)]
mod tests;
