//! STUB FEATURE — delete src/features/example to start your project.
//!
//! axum HTTP handlers for the example feature.
//!
//! Routes (mounted under `/api/v1` by the server shell, wave 5):
//!   `POST   /items`       → 201 + `ItemResponse`
//!   `GET    /items`       → 200 + `ListItemsResponse`  (query: `limit`, `offset`)
//!   `GET    /items/:id`   → 200 + `ItemResponse`
//!
//! Generic over `S: domain::Service` so tests inject a `MockService`.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::features::example::domain::{Error, Service};
use crate::features::example::dto::{
    CreateItemRequest, ItemResponse, ListItemsRequest, ListItemsResponse,
};
use crate::shared::errors::AppError;
use validator::Validate;

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

/// Build the axum router for the example feature. The caller is expected to
/// merge this under `/api/v1` (wave 5).
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
    let item = h.service.create(req.name).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(ItemResponse::from_item(&item))))
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
    let items = h
        .service
        .list(req.limit.unwrap_or(0), req.offset.unwrap_or(0))
        .await
        .map_err(AppError::from)?;
    Ok(Json(ListItemsResponse::from(items)))
}

async fn get_one<S>(
    State(h): State<Handler<S>>,
    Path(id): Path<String>,
) -> Result<Json<ItemResponse>, AppError>
where
    S: Service + ?Sized,
{
    let id = Uuid::parse_str(&id).map_err(|_| AppError::from(Error::InvalidId))?;
    let item = h.service.get(id).await.map_err(AppError::from)?;
    Ok(Json(ItemResponse::from_item(&item)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::example::domain::{Item, MockService};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as SC};
    use mockall::predicate::*;
    use time::OffsetDateTime;
    use tower::ServiceExt;

    fn sample(id: Uuid, name: &str) -> Item {
        let now = OffsetDateTime::now_utc();
        Item {
            id,
            name: name.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn router_with(mock: MockService) -> Router {
        routes(Arc::new(mock))
    }

    #[tokio::test]
    async fn post_items_returns_201_on_success() {
        let mut m = MockService::new();
        m.expect_create()
            .withf(|n| n == "alpha")
            .returning(|n| Ok(sample(Uuid::nil(), &n)));
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
        // Handler forwards query params as-is (0,0 here). Defaults are
        // applied by the service impl, not exercised in this handler test.
        m.expect_list()
            .withf(|l, o| *l == 0 && *o == 0)
            .returning(|_, _| Ok(vec![sample(Uuid::nil(), "alpha")]));
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
            .with(eq(Uuid::nil()))
            .returning(|_| Ok(sample(Uuid::nil(), "alpha")));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/items/{}", Uuid::nil()))
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
            .with(eq(Uuid::nil()))
            .returning(|_| Err(Error::NotFound));
        let app = router_with(m);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/items/{}", Uuid::nil()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), SC::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_items_by_id_returns_400_on_bad_uuid() {
        let m = MockService::new();
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
