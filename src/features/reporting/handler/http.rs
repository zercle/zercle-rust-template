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
    req.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })?;
    let resp = h.service.summary(req).await.map_err(AppError::from)?;
    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::reporting::contract::{CatalogStats, MachineStats, SalesStats};
    use crate::features::reporting::usecase::MockService;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use tower::ServiceExt;

    fn sample_response() -> SummaryResponse {
        SummaryResponse {
            catalog: CatalogStats {
                product_count: 2,
                total_stock: 9,
            },
            machines: MachineStats {
                machine_count: 1,
                total_coin_bank_cents: 250,
            },
            sales: SalesStats {
                purchase_count: 3,
                revenue_cents: 300,
            },
            top_machines: vec![],
        }
    }

    fn router_with(mock: MockService) -> Router {
        routes(Arc::new(mock))
    }

    #[tokio::test]
    async fn get_summary_returns_200_with_payload() {
        let mut m = MockService::new();
        // The handler forwards the raw contract request; clamping is the use
        // case's job and is covered there.
        m.expect_summary()
            .withf(|req: &SummaryRequest| req.top.is_none())
            .returning(|_| Ok(sample_response()));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/reports/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["catalog"]["product_count"], 2);
        assert_eq!(v["machines"]["total_coin_bank_cents"], 250);
        assert_eq!(v["sales"]["revenue_cents"], 300);
    }

    #[tokio::test]
    async fn get_summary_forwards_top_query_param() {
        let mut m = MockService::new();
        m.expect_summary()
            .withf(|req: &SummaryRequest| req.top == Some(3))
            .returning(|_| Ok(sample_response()));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/reports/summary?top=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
    }

    #[tokio::test]
    async fn get_summary_returns_400_on_invalid_top() {
        // `top=0` fails `validate:"omitempty,min=1"` at the boundary; the
        // service must never be called.
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/reports/summary?top=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }
}
