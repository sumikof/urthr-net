use anchor_lang::prelude::*;
use crate::error::UrthrError;

/// Protocol fee = amount * fee_bps / 10_000, with checked math.
pub fn fee_amount(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee = (amount as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(UrthrError::MathOverflow)?
        .checked_div(crate::constants::FEE_DENOMINATOR as u128)
        .ok_or(UrthrError::MathOverflow)?;
    u64::try_from(fee).map_err(|_| UrthrError::MathOverflow.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_is_bps_of_amount() {
        // 0.5% of 1_000_000 = 5_000
        assert_eq!(fee_amount(1_000_000, 50).unwrap(), 5_000);
    }

    #[test]
    fn zero_bps_is_zero_fee() {
        assert_eq!(fee_amount(1_000_000, 0).unwrap(), 0);
    }

    #[test]
    fn rounds_down() {
        // 1 bps of 12_345 = 1.2345 -> 1
        assert_eq!(fee_amount(12_345, 1).unwrap(), 1);
    }
}
