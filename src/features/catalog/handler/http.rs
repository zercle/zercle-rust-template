//! axum HTTP handler for catalog (Go `handler/handler.go` parity,
//! port-spec §5.1a/§5.1h).
//!
//! Routes (nested under `/api/v1` by the feature's `di`):
//! `POST /products` → 201 + `ProductResponse`
//! `GET /products` → 200 + `ListProductsResponse` (query: `limit`, `offset`)
//! `GET /products/{id}` → 200 + `ProductResponse`
//!
//! Handlers bind the feature's contract types directly, run `validator`, and
//! call the usecase service — never mapping from domain entities.
//! Errors map to the shared [`AppError`] envelope (`IntoResponse`). Generic
//! over `S: Service` so tests inject a `MockService`.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use validator::Validate;

use crate::features::catalog::contract::{
    CreateProductRequest, ListProductsRequest, ListProductsResponse, ProductResponse,
};
use crate::features::catalog::usecase::Service;
use crate::platform::errors::AppError;

/// Handler holds the service as `Arc<S>`; generic keeps the test seam clean.
pub struct Handler<S: Service + ?Sized> {
    service: Arc<S>,
}

// Manual `Clone` impl: `Arc<S>` is `Clone` for any `S`, including `?Sized`.
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

/// Build the axum router for the catalog feature. The caller (the feature's
/// `di`) nests it under `/api/v1`.
pub fn routes<S>(service: Arc<S>) -> Router
where
    S: Service + ?Sized + Send + Sync + 'static,
{
    let state = Handler::new(service);
    Router::new()
        .route("/products", post(create::<S>).get(list::<S>))
        .route("/products/{id}", get(get_product::<S>))
        .with_state(state)
}

/// `POST /products` — bind + validate, then create; 201 on success.
async fn create<S: Service + ?Sized>(
    State(h): State<Handler<S>>,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), AppError> {
    validate(&req)?;
    let response = h.service.create(req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /products` — bind + validate pagination, then list; 200 on success.
async fn list<S: Service + ?Sized>(
    State(h): State<Handler<S>>,
    Query(req): Query<ListProductsRequest>,
) -> Result<Json<ListProductsResponse>, AppError> {
    validate(&req)?;
    Ok(Json(h.service.list(req).await.map_err(AppError::from)?))
}

/// `GET /products/{id}` — fetch by path id; 200 on success, 404 when absent.
async fn get_product<S: Service + ?Sized>(
    State(h): State<Handler<S>>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    Ok(Json(h.service.get(id).await.map_err(AppError::from)?))
}

/// Map a `validator` failure to the shared `INVALID_INPUT` boundary error.
fn validate<T: Validate>(value: &T) -> Result<(), AppError> {
    value.validate().map_err(AppError::from_validation)
}

#[cfg(test)]
mod tests;
