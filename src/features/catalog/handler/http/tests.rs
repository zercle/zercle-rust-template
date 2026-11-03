use super::*;
use crate::features::catalog::usecase::MockService;
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
