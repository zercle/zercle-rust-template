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
    req.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })?;
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
    req.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })?;
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
    req.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })?;
    let resp = h
        .service
        .restock_bank(id, req)
        .await
        .map_err(AppError::from)?;
    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::machines::domain::Error;
    use crate::features::machines::usecase::MockService;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use mockall::predicate::*;
    use tower::ServiceExt;

    fn sample_response(id: &str, label: &str) -> MachineResponse {
        let mut coin_bank = std::collections::BTreeMap::new();
        coin_bank.insert(25, 2);
        MachineResponse {
            id: id.to_string(),
            label: label.to_string(),
            coin_bank,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    fn router_with(mock: MockService) -> Router {
        routes(Arc::new(mock))
    }

    #[tokio::test]
    async fn post_machines_returns_201_on_success() {
        let mut m = MockService::new();
        m.expect_create()
            .withf(|req| req.label == "lobby" && req.initial_coins == vec![25, 25])
            .returning(|req| Ok(sample_response("id-1", &req.label)));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/machines")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"lobby","initial_coins":[25,25]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["coin_bank"]["25"], 2);
    }

    #[tokio::test]
    async fn post_machines_returns_400_on_empty_label() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/machines")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"","initial_coins":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_machines_returns_400_on_non_positive_coin() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/machines")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"lobby","initial_coins":[0]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_machines_returns_200_with_payload() {
        let mut m = MockService::new();
        // The handler forwards the raw contract request; defaults/clamping are
        // the use case's job and are covered there.
        m.expect_list()
            .withf(|req: &ListMachinesRequest| req.limit.is_none() && req.offset.is_none())
            .returning(|_| {
                Ok(ListMachinesResponse {
                    machines: vec![sample_response("id-1", "lobby")],
                })
            });
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/machines")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["machines"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_machines_returns_400_on_negative_limit() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/machines?limit=-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_machines_by_id_returns_200_on_hit() {
        let mut m = MockService::new();
        m.expect_get()
            .withf(|id| id == "00000000-0000-0000-0000-000000000000")
            .returning(|id| Ok(sample_response(&id, "lobby")));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/machines/{}", uuid::Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
    }

    #[tokio::test]
    async fn get_machines_by_id_returns_404_on_missing() {
        let mut m = MockService::new();
        m.expect_get().returning(|_| Err(Error::MachineNotFound));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/machines/{}", uuid::Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_machines_by_id_returns_400_on_bad_uuid() {
        let mut m = MockService::new();
        m.expect_get().returning(|_| Err(Error::InvalidId));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/machines/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_machines_bank_returns_200_on_success() {
        let mut m = MockService::new();
        m.expect_restock_bank()
            .withf(|id, req| id == "00000000-0000-0000-0000-000000000000" && req.coins == vec![25])
            .returning(|id, _| Ok(sample_response(&id, "lobby")));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/machines/{}/bank", uuid::Uuid::nil()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"coins":[25]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
    }

    #[tokio::test]
    async fn post_machines_bank_returns_400_on_empty_coins() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/machines/{}/bank", uuid::Uuid::nil()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"coins":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }
}
