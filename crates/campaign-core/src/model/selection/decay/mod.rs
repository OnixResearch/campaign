//! Representability of selection-count decay under the admitted exploration bounds.

/// Magnitude of the lowest `i64` rank: `|i64::MIN|`, that is `2^63`.
const RANK_FLOOR_MAGNITUDE: u64 = i64::MIN.unsigned_abs();

impl super::SelectionCountDecay {
    /// Checks that decayed first ranks stay `i64` values under `bounds`.
    ///
    /// A candidate is decayed by at most `max_selections - 1` prior selections, and its first rank is at
    /// least `-min(max_rank_magnitude, 2^63)`, so `subtract_from_first_rank` is representable exactly
    /// when `penalty_per_selection * (max_selections - 1) + min(max_rank_magnitude, 2^63) <= 2^63`
    /// (checked). `None` is always representable.
    ///
    /// # Errors
    ///
    /// Returns `InvalidConfig` when that sum overflows or exceeds `2^63`, or `max_selections` is zero.
    // r[impl campaign.selection.decay]
    pub fn validate_within_bounds(
        &self,
        bounds: &crate::model::ExplorationBounds,
    ) -> crate::error::CampaignResult<()> {
        let Self::SubtractFromFirstRank {
            penalty_per_selection,
        } = self
        else {
            return Ok(());
        };
        let prior_selections = bounds
            .max_selections
            .checked_sub(1)
            .ok_or(crate::error::CampaignError::InvalidConfig)?;
        let lowest_rank_magnitude = bounds.max_rank_magnitude.min(RANK_FLOOR_MAGNITUDE);
        let lowest_decayed_magnitude = penalty_per_selection
            .checked_mul(prior_selections)
            .and_then(|penalty| penalty.checked_add(lowest_rank_magnitude))
            .ok_or(crate::error::CampaignError::InvalidConfig)?;
        if lowest_decayed_magnitude > RANK_FLOOR_MAGNITUDE {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        debug_assert!(lowest_rank_magnitude <= lowest_decayed_magnitude);
        debug_assert!(lowest_decayed_magnitude <= RANK_FLOOR_MAGNITUDE);
        Ok(())
    }
}
