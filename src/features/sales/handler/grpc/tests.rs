use super::*;
use crate::features::sales::domain::Error as DomainError;
use crate::features::sales::usecase::MockService;

fn sample() -> PurchaseResponse {
    PurchaseResponse {
        id: "id".to_string(),
        machine_id: "m".to_string(),
        product_id: "p".to_string(),
        price_cents: 25,
        total_inserted_cents: 100,
        change_cents: 75,
        change_coins: vec![50, 25],
        purchased_at: "1970-01-01T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn purchase_ok() {
    let mut m = MockService::new();
    m.expect_purchase()
        .withf(|req| req.machine_id == "m" && req.coins == vec![100])
        .returning(|_| Ok(sample()));
    let srv = GrpcServer::new(Arc::new(m));
    let resp = srv
        .purchase(Request::new(PbPurchaseRequest {
            machine_id: "m".to_string(),
            product_id: "p".to_string(),
            coins: vec![100],
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.change_cents, 75);
    assert_eq!(resp.change_coins, vec![50, 25]);
}

#[tokio::test]
async fn purchase_maps_domain_errors_to_status_codes() {
    // `domain::Error` is not `Clone` (its `Internal` cause is an `anyhow::Error`),
    // so carry the unit-variant constructors as `fn() -> Error` factories.
    let cases: [(fn() -> DomainError, tonic::Code); 4] = [
        (|| DomainError::MachineNotFound, tonic::Code::NotFound),
        (|| DomainError::OutOfStock, tonic::Code::AlreadyExists),
        (
            || DomainError::UnsupportedCoin,
            tonic::Code::InvalidArgument,
        ),
        (
            || DomainError::InsufficientPayment,
            tonic::Code::InvalidArgument,
        ),
    ];
    for (make, expected) in cases {
        let mut m = MockService::new();
        m.expect_purchase().returning(move |_| Err(make()));
        let srv = GrpcServer::new(Arc::new(m));
        let err = srv
            .purchase(Request::new(PbPurchaseRequest {
                machine_id: "m".to_string(),
                product_id: "p".to_string(),
                coins: vec![25],
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), expected);
    }
}
