use super::*;
use crate::features::catalog::contract::ListProductsResponse;
use crate::features::catalog::usecase::MockService;

fn sample(id: &str, name: &str) -> ProductResponse {
    ProductResponse {
        id: id.to_string(),
        name: name.to_string(),
        price_cents: 150,
        stock: 2,
        created_at: "1970-01-01T00:00:00Z".to_string(),
        updated_at: "1970-01-01T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn create_product_ok() {
    let mut m = MockService::new();
    m.expect_create()
        .withf(|req: &CreateProductRequest| req.name == "alpha")
        .returning(|req| Ok(sample("some-id", &req.name)));
    let srv = GrpcServer::new(Arc::new(m));

    let resp = srv
        .create_product(Request::new(PbCreateProductRequest {
            name: "alpha".to_string(),
            price_cents: 150,
            stock: 2,
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().name, "alpha");
}

#[tokio::test]
async fn create_product_maps_invalid_price_to_invalid_argument() {
    let mut m = MockService::new();
    m.expect_create()
        .returning(|_| Err(crate::features::catalog::domain::Error::InvalidPrice));
    let srv = GrpcServer::new(Arc::new(m));

    let err = srv
        .create_product(Request::new(PbCreateProductRequest {
            name: "alpha".to_string(),
            price_cents: 0,
            stock: 0,
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn get_product_maps_not_found_to_not_found() {
    let mut m = MockService::new();
    m.expect_get()
        .returning(|_| Err(crate::features::catalog::domain::Error::ProductNotFound));
    let srv = GrpcServer::new(Arc::new(m));

    let err = srv
        .get_product(Request::new(GetProductRequest {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn list_products_maps_fields() {
    let mut m = MockService::new();
    m.expect_list()
        .withf(|req: &ListProductsRequest| req.limit == Some(10) && req.offset == Some(0))
        .returning(|_| {
            Ok(ListProductsResponse {
                products: vec![sample("id-1", "a"), sample("id-2", "b")],
            })
        });
    let srv = GrpcServer::new(Arc::new(m));

    let resp = srv
        .list_products(Request::new(PbListProductsRequest {
            limit: 10,
            offset: 0,
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().products.len(), 2);
}
