use super::*;
use crate::features::catalog::repository::MockRepository;
use mockall::predicate::*;

fn product(id: Uuid, name: &str, price_cents: i32) -> Product {
    let now = OffsetDateTime::now_utc();
    Product {
        id,
        name: name.to_string(),
        price_cents,
        stock: 5,
        created_at: now,
        updated_at: now,
    }
}

fn usecase(repo: MockRepository) -> Usecase {
    Usecase::new(Arc::new(repo), 20, 100, 255)
}

#[tokio::test]
async fn create_rejects_empty_name() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateProductRequest {
            name: "   ".to_string(),
            price_cents: 10,
            stock: 1,
        })
        .await
        .unwrap_err(),
        Error::InvalidProductName
    );
}

#[tokio::test]
async fn create_rejects_overlong_name() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateProductRequest {
            name: "a".repeat(256),
            price_cents: 10,
            stock: 1,
        })
        .await
        .unwrap_err(),
        Error::InvalidProductName
    );
}

#[tokio::test]
async fn create_accepts_multibyte_name_within_rune_cap() {
    let mut mock = MockRepository::new();
    mock.expect_create().returning(|_| Ok(()));
    let svc = usecase(mock);
    let name = "ช".repeat(200);
    let resp = svc
        .create(CreateProductRequest {
            name,
            price_cents: 10,
            stock: 1,
        })
        .await
        .unwrap();
    assert_eq!(resp.name.chars().count(), 200);
}

#[tokio::test]
async fn create_rejects_multibyte_name_over_rune_cap() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateProductRequest {
            name: "🎉".repeat(256),
            price_cents: 10,
            stock: 1,
        })
        .await
        .unwrap_err(),
        Error::InvalidProductName
    );
}

#[tokio::test]
async fn create_rejects_non_positive_price() {
    let svc = usecase(MockRepository::new());
    for price in [0, -1] {
        assert_eq!(
            svc.create(CreateProductRequest {
                name: "cola".to_string(),
                price_cents: price,
                stock: 1,
            })
            .await
            .unwrap_err(),
            Error::InvalidPrice
        );
    }
}

#[tokio::test]
async fn create_trims_and_persists() {
    let mut mock = MockRepository::new();
    mock.expect_create().returning(|_| Ok(()));
    let svc = usecase(mock);
    let resp = svc
        .create(CreateProductRequest {
            name: "  cola  ".to_string(),
            price_cents: 150,
            stock: 2,
        })
        .await
        .unwrap();
    assert_eq!(resp.name, "cola");
    assert_eq!(resp.price_cents, 150);
}

#[tokio::test]
async fn create_maps_domain_to_wire_timestamps() {
    let mut mock = MockRepository::new();
    mock.expect_create().returning(|_| Ok(()));
    let svc = usecase(mock);
    let resp = svc
        .create(CreateProductRequest {
            name: "alpha".to_string(),
            price_cents: 10,
            stock: 0,
        })
        .await
        .unwrap();
    assert!(resp.created_at.ends_with('Z'), "got {}", resp.created_at);
    assert!(Uuid::parse_str(&resp.id).is_ok());
}

#[tokio::test]
async fn get_rejects_malformed_id_before_touching_the_port() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.get("not-a-uuid".to_string()).await.unwrap_err(),
        Error::InvalidId
    );
}

#[tokio::test]
async fn get_passes_through_not_found() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id()
        .with(eq(Uuid::nil()))
        .returning(|_| Err(Error::ProductNotFound));
    let svc = usecase(mock);
    assert_eq!(
        svc.get(Uuid::nil().to_string()).await.unwrap_err(),
        Error::ProductNotFound
    );
}

#[tokio::test]
async fn get_returns_wire_response_on_hit() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id()
        .with(eq(Uuid::nil()))
        .returning(|id| Ok(product(id, "x", 42)));
    let svc = usecase(mock);
    let got = svc.get(Uuid::nil().to_string()).await.unwrap();
    assert_eq!(got.name, "x");
    assert_eq!(got.price_cents, 42);
    assert_eq!(got.id, Uuid::nil().to_string());
}

#[tokio::test]
async fn list_clamps_limit_above_max() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 100 && *offset == 0)
        .returning(|_, _| Ok(vec![]));
    let svc = usecase(mock);
    let resp = svc
        .list(ListProductsRequest {
            limit: Some(9_999),
            offset: Some(0),
        })
        .await
        .unwrap();
    assert!(resp.products.is_empty());
}

#[tokio::test]
async fn list_uses_default_when_limit_zero_or_missing() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 20 && *offset == 0)
        .returning(|_, _| Ok(vec![]))
        .times(2);
    let svc = usecase(mock);
    svc.list(ListProductsRequest {
        limit: None,
        offset: None,
    })
    .await
    .unwrap();
    svc.list(ListProductsRequest {
        limit: Some(0),
        offset: Some(0),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn list_clamps_negative_offset_to_zero() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 10 && *offset == 0)
        .returning(|_, _| Ok(vec![]));
    let svc = usecase(mock);
    svc.list(ListProductsRequest {
        limit: Some(10),
        offset: Some(-5),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn fallback_defaults_apply_when_config_zero() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 100 && *offset == 0)
        .returning(|_, _| Ok(vec![]));
    let svc = Usecase::new(Arc::new(mock), 0, 0, 0);
    svc.list(ListProductsRequest {
        limit: Some(9_999),
        offset: Some(0),
    })
    .await
    .unwrap();
}
