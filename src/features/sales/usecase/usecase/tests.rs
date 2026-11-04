use super::*;
use crate::features::sales::domain::{CoinBank, SaleProduct};
use crate::features::sales::repository::MockRepository;

fn bank(pairs: &[(i32, i32)]) -> CoinBank {
    pairs.iter().copied().collect()
}

fn request(machine: Uuid, product: Uuid, coins: Vec<i32>) -> PurchaseRequest {
    PurchaseRequest {
        machine_id: machine.to_string(),
        product_id: product.to_string(),
        coins,
    }
}

fn usecase(repo: MockRepository) -> Usecase {
    Usecase::new(Arc::new(repo))
}

#[tokio::test]
async fn purchase_rejects_malformed_machine_id_before_port() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(PurchaseRequest {
            machine_id: "bad".to_string(),
            product_id: Uuid::nil().to_string(),
            coins: vec![25],
        })
        .await
        .unwrap_err(),
        Error::InvalidId
    );
}

#[tokio::test]
async fn purchase_rejects_malformed_product_id_before_port() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(PurchaseRequest {
            machine_id: Uuid::nil().to_string(),
            product_id: "bad".to_string(),
            coins: vec![25],
        })
        .await
        .unwrap_err(),
        Error::InvalidId
    );
}

#[tokio::test]
async fn purchase_rejects_unsupported_coin_before_reads() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(request(Uuid::nil(), Uuid::nil(), vec![7]))
            .await
            .unwrap_err(),
        Error::UnsupportedCoin
    );
}

#[tokio::test]
async fn purchase_propagates_product_not_found() {
    let mut mock = MockRepository::new();
    mock.expect_get_product()
        .returning(|_| Err(Error::ProductNotFound));
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(request(Uuid::nil(), Uuid::nil(), vec![25]))
            .await
            .unwrap_err(),
        Error::ProductNotFound
    );
}

#[tokio::test]
async fn purchase_rejects_insufficient_payment() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().returning(|id| {
        Ok(SaleProduct {
            product_id: id,
            price_cents: 100,
            stock: 5,
        })
    });
    mock.expect_get_machine_bank()
        .returning(|_| Ok(bank(&[(25, 4)])));
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(request(Uuid::nil(), Uuid::nil(), vec![25]))
            .await
            .unwrap_err(),
        Error::InsufficientPayment
    );
}

#[tokio::test]
async fn purchase_rejects_out_of_stock() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().returning(|id| {
        Ok(SaleProduct {
            product_id: id,
            price_cents: 25,
            stock: 0,
        })
    });
    mock.expect_get_machine_bank()
        .returning(|_| Ok(bank(&[(25, 4)])));
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(request(Uuid::nil(), Uuid::nil(), vec![25]))
            .await
            .unwrap_err(),
        Error::OutOfStock
    );
}

#[tokio::test]
async fn purchase_exact_payment_commits_empty_change() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().returning(|id| {
        Ok(SaleProduct {
            product_id: id,
            price_cents: 50,
            stock: 5,
        })
    });
    mock.expect_get_machine_bank()
        .returning(|_| Ok(bank(&[(25, 4)])));
    mock.expect_commit_purchase()
        .withf(|_, _, record, _| record.change_cents == 0 && record.change_coins.is_empty())
        .returning(|_, _, _, _| Ok(()));
    let svc = usecase(mock);
    let resp = svc
        .purchase(request(Uuid::nil(), Uuid::nil(), vec![25, 25]))
        .await
        .unwrap();
    assert_eq!(resp.total_inserted_cents, 50);
    assert_eq!(resp.change_cents, 0);
    assert!(resp.change_coins.is_empty());
}

#[tokio::test]
async fn purchase_overpay_composes_change_and_commits() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().returning(|id| {
        Ok(SaleProduct {
            product_id: id,
            price_cents: 25,
            stock: 5,
        })
    });
    mock.expect_get_machine_bank()
        .returning(|_| Ok(bank(&[(25, 3), (50, 1)])));
    mock.expect_commit_purchase()
        .withf(|_, _, record, bank_after| {
            record.change_cents == 75
                && record.change_coins == vec![50, 25]
                && bank_after.get(&25) == Some(&2)
        })
        .returning(|_, _, _, _| Ok(()));
    let svc = usecase(mock);
    let resp = svc
        .purchase(request(Uuid::nil(), Uuid::nil(), vec![100]))
        .await
        .unwrap();
    assert_eq!(resp.total_inserted_cents, 100);
    assert_eq!(resp.change_cents, 75);
    assert_eq!(resp.change_coins, vec![50, 25]);
}

#[tokio::test]
async fn purchase_propagates_commit_out_of_stock_race() {
    let mut mock = MockRepository::new();
    mock.expect_get_product().returning(|id| {
        Ok(SaleProduct {
            product_id: id,
            price_cents: 25,
            stock: 5,
        })
    });
    mock.expect_get_machine_bank()
        .returning(|_| Ok(bank(&[(25, 4)])));
    mock.expect_commit_purchase()
        .returning(|_, _, _, _| Err(Error::OutOfStock));
    let svc = usecase(mock);
    assert_eq!(
        svc.purchase(request(Uuid::nil(), Uuid::nil(), vec![25]))
            .await
            .unwrap_err(),
        Error::OutOfStock
    );
}
