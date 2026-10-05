//! Sales coin arithmetic (Go `domain/coins.go` parity, port-spec §5.3c).

use std::collections::BTreeMap;

use super::error::Error;

/// Accepted coin denominations in cents, smallest-first (Go
/// `SupportedDenominations`).
pub const SUPPORTED_DENOMINATIONS: [i32; 5] = [5, 10, 25, 50, 100];

/// Denomination in cents → count held (Go `CoinBank map[int32]int32`).
pub type CoinBank = BTreeMap<i32, i32>;

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

/// Sum of inserted coins in cents (Go `SumCoins`).
pub fn sum_coins(coins: &[i32]) -> i32 {
    coins.iter().sum()
}

/// Greedy largest-first change over the canonical `{5,10,25,50,100}` coin
/// system (exact for this system, Go doc note). A non-positive amount yields
/// empty change and an unchanged bank clone. Leftover → `ErrExactChangeRequired`.
pub fn make_change(bank: &CoinBank, amount: i32) -> Result<(Vec<i32>, CoinBank), Error> {
    let mut remaining = bank.clone();
    if amount <= 0 {
        return Ok((Vec::new(), remaining));
    }
    let mut change = Vec::new();
    let mut left = amount;
    for denom in SUPPORTED_DENOMINATIONS.iter().rev() {
        while left >= *denom {
            let count = remaining.get(denom).copied().unwrap_or(0);
            if count == 0 {
                break;
            }
            change.push(*denom);
            left -= *denom;
            if count == 1 {
                remaining.remove(denom);
            } else {
                remaining.insert(*denom, count - 1);
            }
        }
    }
    if left != 0 {
        return Err(Error::ExactChangeRequired);
    }
    Ok((change, remaining))
}

fn is_supported_denomination(coin: i32) -> bool {
    SUPPORTED_DENOMINATIONS.contains(&coin)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank(pairs: &[(i32, i32)]) -> CoinBank {
        pairs.iter().copied().collect()
    }

    #[test]
    fn validate_coins_rejects_unsupported() {
        assert!(validate_coins(&[5, 10, 25, 50, 100]).is_ok());
        assert_eq!(validate_coins(&[7]), Err(Error::UnsupportedCoin));
    }

    #[test]
    fn sum_coins_adds() {
        assert_eq!(sum_coins(&[25, 25, 50]), 100);
        assert_eq!(sum_coins(&[]), 0);
    }

    #[test]
    fn make_change_non_positive_amount_is_empty() {
        let b = bank(&[(25, 2)]);
        let (change, remaining) = make_change(&b, 0).unwrap();
        assert!(change.is_empty());
        assert_eq!(remaining, b);
    }

    #[test]
    fn make_change_greedy_largest_first() {
        // 75 = 50 + 25 (not 3x25).
        let b = bank(&[(25, 3), (50, 1)]);
        let (change, remaining) = make_change(&b, 75).unwrap();
        assert_eq!(change, vec![50, 25]);
        assert_eq!(remaining.get(&25), Some(&2));
        assert_eq!(remaining.get(&50), None);
    }

    #[test]
    fn make_change_capped_by_bank() {
        // Amount 75 but only two quarters: 50 then one 25 leaves 0.
        let b = bank(&[(25, 3)]);
        let (change, _) = make_change(&b, 50).unwrap();
        assert_eq!(change, vec![25, 25]);
    }

    #[test]
    fn make_change_leftover_errors() {
        // 30 with no 5s and no 10s: 25 leaves 5 -> exact change required.
        let b = bank(&[(25, 1)]);
        assert_eq!(make_change(&b, 30), Err(Error::ExactChangeRequired));
    }
}
