//! Machine entity + coin-bank rules (Go `domain/machine.go` parity, port-spec
//! §5.2c).

use std::collections::BTreeMap;

use time::OffsetDateTime;
use uuid::Uuid;

use super::error::Error;

/// Accepted coin denominations in cents, largest-last (Go `Denominations`).
pub const DENOMINATIONS: [i32; 5] = [5, 10, 25, 50, 100];

/// Denomination in cents → count held (Go `CoinBank map[int32]int32`).
pub type CoinBank = BTreeMap<i32, i32>;

/// A vending machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub id: Uuid,
    pub label: String,
    pub coin_bank: CoinBank,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Reject any coin that is not a supported denomination, wrapping
/// `ErrUnsupportedCoin` (Go `ValidateCoins`).
pub fn validate_coins(coins: &[i32]) -> Result<(), Error> {
    for coin in coins {
        if !is_supported_denomination(*coin) {
            return Err(Error::UnsupportedCoin);
        }
    }
    Ok(())
}

/// Return a new bank with each coin added; the input bank is never mutated
/// (Go `AddCoins`).
pub fn add_coins(bank: &CoinBank, coins: &[i32]) -> CoinBank {
    let mut out = bank.clone();
    for coin in coins {
        *out.entry(*coin).or_insert(0) += 1;
    }
    out
}

fn is_supported_denomination(coin: i32) -> bool {
    DENOMINATIONS.contains(&coin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_coins_accepts_supported_denominations() {
        assert!(validate_coins(&[5, 10, 25, 50, 100]).is_ok());
        assert!(validate_coins(&[]).is_ok());
    }

    #[test]
    fn validate_coins_rejects_unsupported() {
        assert_eq!(validate_coins(&[7]), Err(Error::UnsupportedCoin));
        assert_eq!(validate_coins(&[0]), Err(Error::UnsupportedCoin));
        assert_eq!(validate_coins(&[-5]), Err(Error::UnsupportedCoin));
    }

    #[test]
    fn add_coins_increments_and_never_mutates_input() {
        let mut bank = CoinBank::new();
        bank.insert(25, 1);
        let after = add_coins(&bank, &[25, 25, 100]);
        assert_eq!(after.get(&25), Some(&3));
        assert_eq!(after.get(&100), Some(&1));
        assert_eq!(bank.get(&25), Some(&1), "input bank unchanged");
    }
}
