//! Sales use-case implementation (Go `usecase/usecase.go` parity, port-spec
//! §5.3d). Flow:
//!
//! 1. parse `machine_id` then `product_id` (`InvalidId` on malformed)
//! 2. `domain::validate_coins`
//! 3. `repo.get_product` then `repo.get_machine_bank`
//! 4. `domain::purchase` decides insufficiency / stock / exact change
//! 5. build the `PurchaseRecord`, `repo.commit_purchase`, map to contract
//!
//! Errors keep their domain sentinel identity through `?` (Go wraps with
//! `fmt.Errorf("...: %w", err)`; `errors.Is` identity is what matters).

use std::sync::Arc;

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::features::sales::application::Service;
use crate::features::sales::contract::{PurchaseRequest, PurchaseResponse};
use crate::features::sales::domain::{Error, PurchaseRecord, purchase, sum_coins, validate_coins};
use crate::features::sales::port::Repository;

/// Concrete sales use case backed by a [`Repository`] outbound port.
#[derive(Clone)]
pub struct Usecase {
    repo: Arc<dyn Repository>,
}

impl Usecase {
    /// Build a use case. Sales has no configurable limits (Go `NewUsecase(repo)`).
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }

    /// Map a purchase record to its wire form (RFC 3339 timestamp).
    fn purchase_response(record: &PurchaseRecord) -> PurchaseResponse {
        PurchaseResponse {
            id: record.id.to_string(),
            machine_id: record.machine_id.to_string(),
            product_id: record.product_id.to_string(),
            price_cents: record.price_cents,
            total_inserted_cents: record.total_inserted_cents,
            change_cents: record.change_cents,
            change_coins: record.change_coins.clone(),
            purchased_at: format_rfc3339(record.purchased_at),
        }
    }
}

#[async_trait::async_trait]
impl Service for Usecase {
    async fn purchase(&self, req: PurchaseRequest) -> Result<PurchaseResponse, Error> {
        // Cheapest-first fail-fast: both ids parse before any repo call, coins
        // validate before reads (Go §5.3c).
        let machine_id = Uuid::parse_str(&req.machine_id).map_err(|_| Error::InvalidId)?;
        let product_id = Uuid::parse_str(&req.product_id).map_err(|_| Error::InvalidId)?;
        validate_coins(&req.coins)?;

        let product = self.repo.get_product(product_id).await?;
        let bank = self.repo.get_machine_bank(machine_id).await?;
        let (change, remaining) = purchase(product.price_cents, product.stock, &bank, &req.coins)?;

        let record = PurchaseRecord {
            id: Uuid::now_v7(),
            machine_id,
            product_id,
            price_cents: product.price_cents,
            total_inserted_cents: sum_coins(&req.coins),
            change_cents: sum_coins(&change),
            change_coins: change,
            purchased_at: OffsetDateTime::now_utc(),
        };
        self.repo
            .commit_purchase(machine_id, product_id, &record, &remaining)
            .await?;
        Ok(Self::purchase_response(&record))
    }
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::sales::domain::{CoinBank, SaleProduct};
    use crate::features::sales::port::MockRepository;

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
}
