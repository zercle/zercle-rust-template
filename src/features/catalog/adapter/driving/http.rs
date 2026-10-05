//! axum HTTP driving adapter for catalog (Go `handler/handler.go` parity,
//! port-spec §5.1a/§5.1h).
//!
//! Routes (nested under `/api/v1` by the feature's `di`):
//! `POST /products` → 201 + `ProductResponse`
//! `GET /products` → 200 + `ListProductsResponse` (query: `limit`, `offset`)
//! `GET /products/{id}` → 200 + `ProductResponse`
//!
//! Handlers bind the feature's contract types directly, run `validator`, and
//! call the application's inbound port — never mapping from domain entities.
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

use crate::features::catalog::application::Service;
use crate::features::catalog::contract::{
    CreateProductRequest, ListProductsRequest, ListProductsResponse, ProductResponse,
};
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
    value.validate().map_err(|e| AppError::InvalidInput {
        cause: Some(anyhow::Error::msg(e.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::catalog::application::MockService;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use tower::ServiceExt;

    fn sample_response(id: &str, name: &str) -> ProductResponse {
        ProductResponse {
            id: id.to_string(),
            name: name.to_string(),
            price_cents: 150,
            stock: 2,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    fn router_with(mock: MockService) -> Router {
        routes(Arc::new(mock))
    }

    #[tokio::test]
    async fn post_products_returns_201_on_success() {
        let mut m = MockService::new();
        m.expect_create()
            .withf(|req| req.name == "alpha")
            .returning(|req| {
                Ok(sample_response(
                    "00000000-0000-0000-0000-000000000000",
                    &req.name,
                ))
            });
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"alpha","price_cents":150,"stock":2}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::CREATED);
    }

    #[tokio::test]
    async fn post_products_returns_400_on_invalid_price() {
        let m = MockService::new();
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"alpha","price_cents":0,"stock":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "INVALID_INPUT");
    }

    #[tokio::test]
    async fn get_products_returns_200_with_payload() {
        let mut m = MockService::new();
        m.expect_list()
            .withf(|req| req.limit.is_none() && req.offset.is_none())
            .returning(|_| {
                Ok(ListProductsResponse {
                    products: vec![sample_response("id-1", "alpha")],
                })
            });
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["products"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_products_returns_400_on_negative_limit() {
        let m = MockService::new();
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/products?limit=-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_product_returns_200_on_hit() {
        let mut m = MockService::new();
        m.expect_get()
            .with(mockall::predicate::eq(
                "00000000-0000-0000-0000-000000000000".to_string(),
            ))
            .returning(|_| {
                Ok(sample_response(
                    "00000000-0000-0000-0000-000000000000",
                    "alpha",
                ))
            });
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/products/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
    }

    #[tokio::test]
    async fn get_product_returns_404_envelope_on_not_found() {
        let mut m = MockService::new();
        m.expect_get()
            .returning(|_| Err(crate::features::catalog::domain::Error::ProductNotFound));
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/products/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "NOT_FOUND");
        assert_eq!(v["message"], "resource not found");
    }

    #[tokio::test]
    async fn get_product_returns_400_on_malformed_id() {
        let mut m = MockService::new();
        m.expect_get()
            .returning(|_| Err(crate::features::catalog::domain::Error::InvalidId));
        let app = router_with(m);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/products/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }
}
