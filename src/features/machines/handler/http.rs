//! axum HTTP handler for machines (Go `handler/handler.go` parity,
//! port-spec §5.2a/§5.2h).
//!
//! Routes (nested under `/api/v1` by the feature's `di`):
//!   `POST   /machines`            → 201 + `MachineResponse`
//!   `GET    /machines`            → 200 + `ListMachinesResponse` (query: `limit`, `offset`)
//!   `GET    /machines/{id}`       → 200 + `MachineResponse`
//!   `POST   /machines/{id}/bank`  → 200 + `MachineResponse`
//!
//! Handlers bind the feature's contract types directly, validate, and call the
//! usecase service — never the outbound repository. Generic over
//! `S: Service` so tests inject a `MockService`.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use validator::Validate;

use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, ListMachinesResponse, MachineResponse,
    RestockBankRequest,
};
use crate::features::machines::usecase::Service;
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

/// Build the axum router for machines. The caller (the feature's `di`) nests
/// this under `/api/v1`.
pub fn routes<S>(service: Arc<S>) -> Router
where
    S: Service + ?Sized + Send + Sync + 'static,
{
    let state = Handler::new(service);
    Router::new()
        .route("/machines", post(create::<S>))
        .route("/machines", get(list::<S>))
        .route("/machines/{id}", get(get_one::<S>))
        .route("/machines/{id}/bank", post(restock::<S>))
        .with_state(state)
}

async fn create<S>(
    State(h): State<Handler<S>>,
    Json(req): Json<CreateMachineRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: Service + ?Sized,
{
    req.validate().map_err(AppError::from_validation)?;
    let resp = h.service.create(req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(resp)))
}

async fn list<S>(
    State(h): State<Handler<S>>,
    Query(req): Query<ListMachinesRequest>,
) -> Result<Json<ListMachinesResponse>, AppError>
where
    S: Service + ?Sized,
{
    req.validate().map_err(AppError::from_validation)?;
    let resp = h.service.list(req).await.map_err(AppError::from)?;
    Ok(Json(resp))
}

async fn get_one<S>(
    State(h): State<Handler<S>>,
    Path(id): Path<String>,
) -> Result<Json<MachineResponse>, AppError>
where
    S: Service + ?Sized,
{
    // Malformed ids surface as `domain::Error::InvalidId` from the use case —
    // both handlers share the one validation path.
    let resp = h.service.get(id).await.map_err(AppError::from)?;
    Ok(Json(resp))
}

async fn restock<S>(
    State(h): State<Handler<S>>,
    Path(id): Path<String>,
    Json(req): Json<RestockBankRequest>,
) -> Result<Json<MachineResponse>, AppError>
where
    S: Service + ?Sized,
{
    req.validate().map_err(AppError::from_validation)?;
    let resp = h
        .service
        .restock_bank(id, req)
        .await
        .map_err(AppError::from)?;
    Ok(Json(resp))
}

#[cfg(test)]
mod tests;
