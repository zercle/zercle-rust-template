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

use crate::features::sales::contract::{PurchaseRequest, PurchaseResponse};
use crate::features::sales::domain::{Error, PurchaseRecord, purchase, sum_coins, validate_coins};
use crate::features::sales::repository::Repository;
use crate::features::sales::usecase::Service;

/// Concrete sales use case backed by a [`Repository`] outbound interface.
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
mod tests;
