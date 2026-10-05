//! Sales purchase rule + records (Go `domain/purchase.go` parity, port-spec
//! §5.3c).

use time::OffsetDateTime;
use uuid::Uuid;

use super::coins::{CoinBank, make_change, sum_coins, validate_coins};
use super::error::Error;

/// A persisted purchase (Go `domain.PurchaseRecord`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseRecord {
    pub id: Uuid,
    pub machine_id: Uuid,
    pub product_id: Uuid,
    pub price_cents: i32,
    pub total_inserted_cents: i32,
    pub change_cents: i32,
    pub change_coins: Vec<i32>,
    pub purchased_at: OffsetDateTime,
}

/// The product fields a purchase needs (Go `domain.SaleProduct`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleProduct {
    pub product_id: Uuid,
    pub price_cents: i32,
    pub stock: i32,
}

/// Decide a purchase against the read price/stock/bank.
///
/// Order (Go `domain.Purchase`): validate coins first (so an unsupported coin
/// is rejected even when payment suffices), then insufficiency, then stock,
/// then change. Returns `(change, remaining_bank)`.
pub fn purchase(
    price_cents: i32,
    stock: i32,
    bank: &CoinBank,
    coins: &[i32],
) -> Result<(Vec<i32>, CoinBank), Error> {
    validate_coins(coins)?;
    let total = sum_coins(coins);
    if total < price_cents {
        return Err(Error::InsufficientPayment);
    }
    if stock <= 0 {
        return Err(Error::OutOfStock);
    }
    let change_amount = total - price_cents;
    if change_amount == 0 {
        return Ok((Vec::new(), bank.clone()));
    }
    make_change(bank, change_amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn bank(pairs: &[(i32, i32)]) -> CoinBank {
        pairs.iter().copied().collect::<BTreeMap<_, _>>()
    }

    #[test]
    fn purchase_rejects_unsupported_coin_even_when_payment_suffices() {
        let b = bank(&[(25, 4)]);
        assert_eq!(purchase(25, 5, &b, &[7]), Err(Error::UnsupportedCoin));
    }

    #[test]
    fn purchase_rejects_insufficient_payment() {
        let b = bank(&[(25, 4)]);
        assert_eq!(purchase(100, 5, &b, &[25]), Err(Error::InsufficientPayment));
    }

    #[test]
    fn purchase_rejects_out_of_stock_after_payment_check() {
        let b = bank(&[(25, 4)]);
        assert_eq!(purchase(25, 0, &b, &[25]), Err(Error::OutOfStock));
    }

    #[test]
    fn purchase_exact_payment_returns_empty_change_and_keeps_bank() {
        let b = bank(&[(25, 2)]);
        let (change, remaining) = purchase(50, 3, &b, &[25, 25]).unwrap();
        assert!(change.is_empty());
        assert_eq!(remaining, b);
    }

    #[test]
    fn purchase_overpay_returns_composed_change() {
        let b = bank(&[(25, 3), (50, 1)]);
        let (change, remaining) = purchase(25, 3, &b, &[100]).unwrap();
        assert_eq!(change, vec![50, 25]);
        assert_eq!(remaining.get(&25), Some(&2));
    }

    #[test]
    fn purchase_exact_change_required() {
        // Price 25, pay 55? unsupported. Use price 30, pay 30 (5,25): change 0.
        // For exact-change failure: price 25, pay 30 is impossible (30 not a
        // denomination); craft price 20 pay 30? 30 unsupported. Use bank
        // without 5s and amount needing 5: price 95, pay 100 -> change 5 but
        // bank has no 5s.
        let b = bank(&[(25, 4)]);
        assert_eq!(purchase(95, 3, &b, &[100]), Err(Error::ExactChangeRequired));
    }
}
