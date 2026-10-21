//! Generated entropy draws: wide index draws in every residue, and explore draws whose narrowed
//! low word or binary64 value lies on the other side of the threshold.

/// Base of every generated index draw (`2^63`): each draw is above `u32::MAX` and keeps its
/// residue only through a full-width mapping.
const INDEX_BASE: u64 = 1 << 63;

/// Explore-index draw step above one byte (`2^8 + 1`); coprime to every eligible count the
/// fixtures present, so its first `count` multiples above the base fall in pairwise distinct
/// residues.
const WIDE_INDEX: u64 = 257;

/// The first value above the low 32-bit word of a draw.
const LOW_WORD: u64 = 1 << 32;

/// Significand bits of a binary64 value: every draw below `2^53` is exactly representable.
const BINARY64_SIGNIFICAND: u32 = 53;

/// Largest exponent the binary64 draw search tries (draws stay below `2^63`).
const BINARY64_TOP_EXPONENT: u32 = 62;

/// Residues searched on one side of the threshold for a draw whose binary64 rounding crosses it.
const BINARY64_SPAN: u64 = 64;

/// Index draws `INDEX_BASE + WIDE_INDEX * k` for `k` in `1..=count`: every draw is above
/// `u32::MAX`, and the draws cover every residue modulo `count` exactly once, so a ticket that
/// drops, truncates, narrows, reorders, or fixes the index draw selects another candidate for at
/// least one of them.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` when a draw is not representable and `Diverged` when two draws
/// share a residue.
pub(super) fn wide_indexes(
    count: usize,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<u64>> {
    let modulus =
        u64::try_from(count).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut draws = alloc::vec::Vec::with_capacity(count);
    for multiple in 1..=modulus {
        draws.push(
            WIDE_INDEX
                .checked_mul(multiple)
                .and_then(|step| INDEX_BASE.checked_add(step))
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        );
    }
    let mut residues: alloc::vec::Vec<u64> = draws
        .iter()
        .filter_map(|draw| draw.checked_rem(modulus))
        .collect();
    residues.sort_unstable();
    residues.dedup();
    super::super::super::common::require(
        residues.len() == count && draws.iter().all(|draw| *draw > u64::from(u32::MAX)),
        "the index draws are above u32::MAX and pairwise distinct modulo the eligible count",
    )?;
    debug_assert_eq!(draws.len(), count);
    Ok(draws)
}

/// An explore draw above `2^32` on the requested side of `threshold` whose low 32-bit word lies
/// on the other side, so a ticket mapping that narrows the explore draw takes the other branch.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` when the draw is not representable and `Diverged` when the
/// generated draw and its low word are not on opposite sides of the threshold.
pub(super) fn beyond_low_word(
    threshold: u64,
    explores: bool,
) -> crate::verdicts::ConformanceResult<u64> {
    let scale = campaign_core::ENTROPY_SCALE;
    let overflow = || campaign_core::CampaignError::ArithmeticOverflow;
    let carry = LOW_WORD.checked_rem(scale).ok_or_else(overflow)?;
    let shortfall = scale.checked_sub(carry).ok_or_else(overflow)?;
    let low = if explores {
        threshold.max(shortfall)
    } else {
        threshold.saturating_sub(carry)
    };
    let draw = LOW_WORD.checked_add(low).ok_or_else(overflow)?;
    let side = |value: u64| value.checked_rem(scale).map(|residue| residue < threshold);
    let word = draw.checked_rem(LOW_WORD).ok_or_else(overflow)?;
    super::super::super::common::require(
        side(draw) == Some(explores) && side(word) == Some(!explores),
        "the wide explore draw and its low word lie on opposite sides of the threshold",
    )?;
    debug_assert!(draw > u64::from(u32::MAX));
    Ok(draw)
}

/// `draw` rounded to the nearest binary64 value (ties to even), as a mapping through `f64` keeps
/// it; `None` when the rounding is not representable.
// r[impl campaign.entropy.valid]
pub(crate) fn binary64(draw: u64) -> Option<u64> {
    let width = u64::BITS.checked_sub(draw.leading_zeros())?;
    let Some(shift) = width
        .checked_sub(BINARY64_SIGNIFICAND)
        .filter(|shift| *shift > 0)
    else {
        return Some(draw);
    };
    let unit = 1_u64.checked_shl(shift)?;
    let remainder = draw & unit.checked_sub(1)?;
    let half = unit.checked_shr(1)?;
    let down = draw.checked_sub(remainder)?;
    let is_even = down
        .checked_shr(shift)
        .is_some_and(|significand| significand & 1 == 0);
    debug_assert!(remainder < unit);
    debug_assert_eq!(down & unit.checked_sub(1)?, 0);
    if remainder < half || (remainder == half && is_even) {
        Some(down)
    } else {
        down.checked_add(unit)
    }
}

/// An explore draw of at least `2^53` on the requested side of `threshold` whose nearest binary64
/// value lies on the other side, so a ticket mapping that carries the explore draw through a
/// binary64 float takes the other branch (and no longer decodes to the draw). The search tries
/// draws near the threshold residue in the first multiples of the entropy scale above each power
/// of two from `2^53`; `None` when no draw there crosses (a threshold whose other side holds only
/// residues no binary64 value of at least `2^53` reaches, such as an exploit side of the single
/// odd residue `999_999`).
// r[impl campaign.entropy.valid]
pub(super) fn beyond_binary64(threshold: u64, explores: bool) -> Option<u64> {
    let scale = campaign_core::ENTROPY_SCALE;
    let side = |value: u64| value.checked_rem(scale).map(|residue| residue < threshold);
    for exponent in BINARY64_SIGNIFICAND..=BINARY64_TOP_EXPONENT {
        let first = 1_u64
            .checked_shl(exponent)
            .and_then(|power| power.checked_div(scale))
            .and_then(|whole| whole.checked_add(1));
        let Some(first) = first else {
            continue;
        };
        for multiple in 0..BINARY64_SPAN {
            let start = first
                .checked_add(multiple)
                .and_then(|whole| whole.checked_mul(scale));
            let Some(start) = start else {
                continue;
            };
            for offset in 0..BINARY64_SPAN {
                let residue = if explores {
                    threshold.checked_sub(offset.saturating_add(1))
                } else {
                    threshold
                        .checked_add(offset)
                        .filter(|residue| *residue < scale)
                };
                let Some(draw) = residue.and_then(|residue| start.checked_add(residue)) else {
                    continue;
                };
                if side(draw) == Some(explores) && binary64(draw).and_then(side) == Some(!explores)
                {
                    debug_assert!(draw >= 1_u64 << BINARY64_SIGNIFICAND);
                    return Some(draw);
                }
            }
        }
    }
    None
}
