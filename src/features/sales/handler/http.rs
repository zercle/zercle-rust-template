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
    req.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })?;
    let resp: PurchaseResponse = h.service.purchase(req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(resp)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::sales::domain::Error as DomainError;
    use crate::features::sales::usecase::MockService;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use tower::ServiceExt;

    fn sample_response() -> PurchaseResponse {
        PurchaseResponse {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            machine_id: "00000000-0000-0000-0000-000000000001".to_string(),
            product_id: "00000000-0000-0000-0000-000000000002".to_string(),
            price_cents: 25,
            total_inserted_cents: 100,
            change_cents: 75,
            change_coins: vec![50, 25],
            purchased_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    fn request(body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/purchases")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn post_purchases_returns_201_on_success() {
        let mut m = MockService::new();
        m.expect_purchase()
            .withf(|req| req.machine_id == "m" && req.coins == vec![100])
            .returning(|_| Ok(sample_response()));
        let app = routes(Arc::new(m));
        let resp = app
            .oneshot(request(
                r#"{"machine_id":"m","product_id":"p","coins":[100]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["change_cents"], 75);
        assert_eq!(v["change_coins"][0], 50);
    }

    #[tokio::test]
    async fn post_purchases_returns_400_on_empty_coins() {
        let m = MockService::new();
        let app = routes(Arc::new(m));
        let resp = app
            .oneshot(request(r#"{"machine_id":"m","product_id":"p","coins":[]}"#))
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_purchases_maps_out_of_stock_to_409_conflict_envelope() {
        let mut m = MockService::new();
        m.expect_purchase()
            .returning(|_| Err(DomainError::OutOfStock));
        let app = routes(Arc::new(m));
        let resp = app
            .oneshot(request(
                r#"{"machine_id":"m","product_id":"p","coins":[25]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::CONFLICT);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "CONFLICT");
        assert_eq!(v["message"], "conflict");
    }

    #[tokio::test]
    async fn post_purchases_maps_unsupported_coin_to_400_invalid_input_envelope() {
        let mut m = MockService::new();
        m.expect_purchase()
            .returning(|_| Err(DomainError::UnsupportedCoin));
        let app = routes(Arc::new(m));
        let resp = app
            .oneshot(request(
                r#"{"machine_id":"m","product_id":"p","coins":[7]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "INVALID_INPUT");
    }

    #[tokio::test]
    async fn post_purchases_maps_unknown_machine_to_404_not_found_envelope() {
        let mut m = MockService::new();
        m.expect_purchase()
            .returning(|_| Err(DomainError::MachineNotFound));
        let app = routes(Arc::new(m));
        let resp = app
            .oneshot(request(
                r#"{"machine_id":"m","product_id":"p","coins":[25]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "NOT_FOUND");
    }
}
