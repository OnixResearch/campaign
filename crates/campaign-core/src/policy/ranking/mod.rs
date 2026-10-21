pub(super) struct EligibleCandidate<'a> {
    pub(super) candidate: &'a crate::model::CandidateState,
    pub(super) guidance: &'a crate::model::GuidanceRecord,
    pub(super) effective_ranks: alloc::vec::Vec<i64>,
}
// r[impl campaign.selection.invalid]
// r[impl campaign.budgets.exhausted]
pub(super) fn eligible_candidates<'a>(
    state: &'a crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<alloc::vec::Vec<crate::policy::ranking::EligibleCandidate<'a>>> {
    let mut out = alloc::vec::Vec::with_capacity(state.candidates.len());
    for candidate in &state.candidates {
        if !matches!(candidate.status, crate::model::CandidateStatus::Retained) {
            continue;
        }
        let Some(guidance) = &candidate.latest_guidance else {
            continue;
        };
        if !crate::policy::capacity::costs_fit(state, &candidate.candidate.declared_costs)? {
            continue;
        }
        let effective_ranks = crate::policy::ranking::apply_decay(
            &guidance.ranks,
            candidate.selection_count,
            &profile.decay,
        )?;
        out.push(crate::policy::ranking::EligibleCandidate {
            candidate,
            guidance,
            effective_ranks,
        });
    }
    out.sort_by(crate::policy::ranking::compare_eligible);
    debug_assert!(out.len() <= state.candidates.len());
    debug_assert!(out.iter().all(|item| {
        matches!(
            item.candidate.status,
            crate::model::CandidateStatus::Retained
        ) && item.candidate.latest_guidance.is_some()
    }));
    Ok(out)
}
// r[impl campaign.entropy.valid]
// r[impl campaign.selection.valid]
pub(super) fn choose_index(
    eligible: &[crate::policy::ranking::EligibleCandidate<'_>],
    profile: &crate::model::SelectionProfile,
    entropy: Option<&crate::model::EntropyTicket>,
) -> crate::error::CampaignResult<usize> {
    if eligible.is_empty() {
        return Err(crate::error::CampaignError::MissingCandidate);
    }
    let crate::model::SelectionMode::Exploratory {
        threshold_parts_per_million,
        exploration_order,
        ..
    } = &profile.mode
    else {
        return Ok(crate::model::RANK_FIRST_DIMENSION);
    };
    let entropy = entropy.ok_or(crate::error::CampaignError::InvalidEntropy)?;
    if entropy.explore_draw() % crate::model::ENTROPY_SCALE >= *threshold_parts_per_million {
        return Ok(crate::model::RANK_FIRST_DIMENSION);
    }
    let order = crate::policy::ranking::exploration_ordering(eligible, exploration_order)?;
    let eligible_count = crate::identity::usize_to_u64(order.len())?;
    let position = entropy
        .index_draw()
        .checked_rem(eligible_count)
        .ok_or(crate::error::CampaignError::MissingCandidate)?;
    let position =
        usize::try_from(position).map_err(|_| crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert_eq!(order.len(), eligible.len());
    order
        .get(position)
        .copied()
        .ok_or(crate::error::CampaignError::MissingCandidate)
}
/// Orders eligible indices by undecayed guidance ranks projected onto the declared dimensions
/// (descending), with candidate identity ascending as the final tie-break.
pub(super) fn exploration_ordering(
    eligible: &[crate::policy::ranking::EligibleCandidate<'_>],
    exploration_order: &[u32],
) -> crate::error::CampaignResult<alloc::vec::Vec<usize>> {
    let mut projected = alloc::vec::Vec::with_capacity(eligible.len());
    for (index, item) in eligible.iter().enumerate() {
        let mut ranks = alloc::vec::Vec::with_capacity(exploration_order.len());
        for dimension in exploration_order {
            let position = usize::try_from(*dimension)
                .map_err(|_| crate::error::CampaignError::ArithmeticOverflow)?;
            let rank = item
                .guidance
                .ranks
                .get(position)
                .copied()
                .ok_or(crate::error::CampaignError::InvalidGuidanceShape)?;
            ranks.push(rank);
        }
        projected.push((ranks, item.candidate.candidate.candidate_id, index));
    }
    projected.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    debug_assert_eq!(projected.len(), eligible.len());
    Ok(projected.into_iter().map(|(_, _, index)| index).collect())
}
/// Total order of eligible candidates: effective ranks descending (lexicographic over the whole
/// vector), then candidate identity ascending. Every admitted rank vector has the campaign's one
/// rank shape, so this is exactly the reference ranking.
// r[impl campaign.guidance.valid]
pub(super) fn compare_eligible(
    left: &crate::policy::ranking::EligibleCandidate<'_>,
    right: &crate::policy::ranking::EligibleCandidate<'_>,
) -> core::cmp::Ordering {
    right
        .effective_ranks
        .cmp(&left.effective_ranks)
        .then_with(|| {
            left.candidate
                .candidate
                .candidate_id
                .cmp(&right.candidate.candidate.candidate_id)
        })
}
// r[impl campaign.selection.decay]
pub(super) fn apply_decay(
    ranks: &[i64],
    count: u64,
    decay: &crate::model::SelectionCountDecay,
) -> crate::error::CampaignResult<alloc::vec::Vec<i64>> {
    let mut out = ranks.to_vec();
    let crate::model::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection,
    } = decay
    else {
        return Ok(out);
    };
    if count == 0 {
        return Ok(out);
    }
    let product = penalty_per_selection
        .checked_mul(count)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    let penalty =
        i64::try_from(product).map_err(|_| crate::error::CampaignError::ArithmeticOverflow)?;
    let first = out
        .get_mut(crate::model::RANK_FIRST_DIMENSION)
        .ok_or(crate::error::CampaignError::InvalidGuidanceShape)?;
    *first = first
        .checked_sub(penalty)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert_ne!(product, 0);
    debug_assert_eq!(out.len(), ranks.len());
    Ok(out)
}
