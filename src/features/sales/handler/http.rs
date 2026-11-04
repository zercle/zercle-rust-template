//! axum HTTP handler for sales (Go `handler/handler.go`
//! parity, port-spec §5.3a/§5.3h).
//!
//! Routes (nested under `/api/v1` by the feature's `di`):
//!   `POST /purchases` → 201 + `PurchaseResponse`
//!
//! The handler binds the feature's contract type directly, validates it, and
//! calls the usecase service — it never maps to or from domain
//! entities. Generic over `S: Service` so tests inject a `MockService`.
//!
//! May reference only the usecase service and contract
//! (`tests/architecture.rs`: handler-ignores-repository).

use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use validator::Validate;

use crate::features::sales::contract::{PurchaseRequest, PurchaseResponse};
use crate::features::sales::usecase::Service;
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

/// Build the axum router for the sales feature. The caller (the feature's
/// `di`) nests this under `/api/v1`.
pub fn routes<S>(service: Arc<S>) -> Router
where
    S: Service + ?Sized + Send + Sync + 'static,
{
    let state = Handler::new(service);
    Router::new()
        .route("/purchases", post(purchase::<S>))
        .with_state(state)
}

async fn purchase<S>(
    State(h): State<Handler<S>>,
    Json(req): Json<PurchaseRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: Service + ?Sized,
{
    req.validate().map_err(AppError::from_validation)?;
    let resp: PurchaseResponse = h.service.purchase(req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(resp)))
}

#[cfg(test)]
mod tests;
