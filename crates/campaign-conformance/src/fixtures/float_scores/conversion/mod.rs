//! Declared fixed-point conversion of non-negative `f64` scores to integer ranks.

/// Fraction bits of the fixed-point rank: `rank = floor(score * 2^FRACTION_BITS)`.
pub const FRACTION_BITS: u32 = 14;

/// IEEE-754 binary64 layout.
const MANTISSA_BITS: u32 = 52;
const EXPONENT_MASK: u64 = 0x7ff;
const MANTISSA_MASK: u64 = 0x000f_ffff_ffff_ffff;
const IMPLICIT_BIT: u64 = 0x0010_0000_0000_0000;
const EXPONENT_BIAS_WITH_MANTISSA: i64 = 1075;
const SUBNORMAL_EXPONENT: i64 = 1;
const MAX_SIGNIFICAND_SHIFT: i64 = 10;

/// Converts one non-negative finite score to its fixed-point rank without floating arithmetic.
///
/// Equal scores give equal ranks, and the conversion is monotone; scores that differ by at least
/// `2^-FRACTION_BITS` give distinct ranks.
///
/// # Errors
///
/// Returns `InvalidGuidanceShape` for a negative, infinite, or not-a-number score and
/// `ArithmeticOverflow` for a score whose rank is not representable.
// r[impl campaign.guidance.float]
pub fn fixed_point_rank(score: f64) -> Result<i64, campaign_core::CampaignError> {
    if !score.is_finite() || score < 0.0 {
        return Err(campaign_core::CampaignError::InvalidGuidanceShape);
    }
    let bits = score.abs().to_bits();
    let mantissa = bits & MANTISSA_MASK;
    let biased = i64::try_from((bits >> MANTISSA_BITS) & EXPONENT_MASK)
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let (significand, exponent) = if biased == 0 {
        (mantissa, SUBNORMAL_EXPONENT)
    } else {
        (mantissa | IMPLICIT_BIT, biased)
    };
    let shift = exponent
        .checked_add(i64::from(FRACTION_BITS))
        .and_then(|shifted| shifted.checked_sub(EXPONENT_BIAS_WITH_MANTISSA))
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let scaled = if shift >= 0 {
        if shift > MAX_SIGNIFICAND_SHIFT {
            return Err(campaign_core::CampaignError::ArithmeticOverflow);
        }
        let left =
            u32::try_from(shift).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
        significand << left
    } else {
        let right = u32::try_from(shift.unsigned_abs())
            .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
        if right >= u64::BITS {
            0
        } else {
            significand >> right
        }
    };
    debug_assert!(significand <= IMPLICIT_BIT | MANTISSA_MASK);
    debug_assert!(shift < 0 || scaled >= significand);
    i64::try_from(scaled).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)
}
