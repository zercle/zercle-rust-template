//! STUB FEATURE — delete src/features/example to start your project.
//!
//! axum HTTP driving adapter for the example feature (Go
//! `adapter/in/http/handler.go` parity).
//!
//! Routes (nested under `/api/v1` by the feature's `di`):
//!   `POST   /items`       → 201 + `ItemResponse`
//!   `GET    /items`       → 200 + `ListItemsResponse`  (query: `limit`, `offset`)
//!   `GET    /items/{id}`  → 200 + `ItemResponse`
//!
//! Handlers bind the feature's contract types directly, validate, and call the
//! application's inbound port — they never map to or from domain entities.
//! Generic over `S: Service` so tests inject a `MockService`.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use validator::Validate;

use crate::features::example::application::Service;
use crate::features::example::contract::{CreateItemRequest, ListItemsRequest, ListItemsResponse};
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

/// Build the axum router for the example feature. The caller (the feature's
/// `di`) nests this under `/api/v1`.
pub fn routes<S>(service: Arc<S>) -> Router
where
    S: Service + ?Sized + Send + Sync + 'static,
{
    let state = Handler::new(service);
    Router::new()
        .route("/items", post(create::<S>))
        .route("/items", get(list::<S>))
        .route("/items/{id}", get(get_one::<S>))
        .with_state(state)
}

async fn create<S>(
    State(h): State<Handler<S>>,
    Json(req): Json<CreateItemRequest>,
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
    Query(req): Query<ListItemsRequest>,
) -> Result<Json<ListItemsResponse>, AppError>
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
) -> Result<Json<crate::features::example::contract::ItemResponse>, AppError>
where
    S: Service + ?Sized,
{
    // Malformed ids surface as `domain::Error::InvalidId` from the use case —
    // both driving adapters share the one validation path.
    let resp = h.service.get(id).await.map_err(AppError::from)?;
    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::example::application::MockService;
    use crate::features::example::contract::ItemResponse;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use mockall::predicate::*;
    use tower::ServiceExt;

    fn sample_response(id: &str, name: &str) -> ItemResponse {
        ItemResponse {
            id: id.to_string(),
            name: name.to_string(),
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    fn router_with(mock: MockService) -> Router {
        routes(Arc::new(mock))
    }

    #[tokio::test]
    async fn post_items_returns_201_on_success() {
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
                    .uri("/items")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"alpha"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::CREATED);
    }

    #[tokio::test]
    async fn post_items_returns_400_on_invalid_name() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/items")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_items_returns_200_with_payload() {
        let mut m = MockService::new();
        // The handler forwards the raw contract request; defaults/clamping are
        // the use case's job and are covered there.
        m.expect_list()
            .withf(|req: &ListItemsRequest| req.limit.is_none() && req.offset.is_none())
            .returning(|_| {
                Ok(ListItemsResponse {
                    items: vec![sample_response("id-1", "alpha")],
                })
            });
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_items_returns_400_on_invalid_limit() {
        let m = MockService::new();
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/items?limit=999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_items_by_id_returns_200_on_hit() {
        let mut m = MockService::new();
        m.expect_get()
            .withf(|id| id == "00000000-0000-0000-0000-000000000000")
            .returning(|id| Ok(sample_response(&id, "alpha")));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/items/{}", uuid::Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::OK);
    }

    #[tokio::test]
    async fn get_items_by_id_returns_404_on_missing() {
        let mut m = MockService::new();
        m.expect_get()
            .returning(|_| Err(crate::features::example::domain::Error::NotFound));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/items/{}", uuid::Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_items_by_id_returns_400_on_bad_uuid() {
        let mut m = MockService::new();
        m.expect_get()
            .returning(|_| Err(crate::features::example::domain::Error::InvalidId));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/items/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::BAD_REQUEST);
    }
}
