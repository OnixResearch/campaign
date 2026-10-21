/// Plans the deterministic prune of retained candidates above `max_retained_candidates`.
///
/// Retained candidates are ordered by their latest guidance ranks projected onto `rank_order`
/// (descending), unguided candidates after every guided one, and candidate identity ascending as
/// the final tie-break. Candidates with outstanding selections always count as kept. The first
/// `max_retained_candidates` stay; up to `max_prune_batch` of the lowest-ordered excess are pruned.
/// Returns `None` when nothing exceeds the retained bound.
///
/// # Errors
///
/// Returns `StaleState` for another campaign or state, `CampaignStopped` after an accepted stop,
/// `InvalidConfig` for an empty or duplicate rank order or one naming a dimension at or beyond
/// `max_rank_dimensions` or the campaign's fixed rank shape, and `InvalidGuidanceShape` when a
/// guided candidate lacks a declared rank dimension.
// r[impl campaign.pruning]
// r[impl campaign.guidance.declared_order]
pub fn plan_pruning(
    state: &crate::frontier::FrontierState,
    request: crate::model::RetentionPlanRequest,
) -> crate::error::CampaignResult<Option<crate::model::PruneRequest>> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    if request.campaign_id != state.admission.campaign_id || request.state_id != state.state_id {
        return Err(crate::error::CampaignError::StaleState);
    }
    request
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    let targets = crate::policy::retention::planning::planned_targets(
        state,
        request.retention_profile_id,
        &request.rank_order,
    )?;
    if targets.is_empty() {
        return Ok(None);
    }
    let mut prune = crate::model::PruneRequest {
        prune_plan_id: crate::identity::Id::absent(),
        campaign_id: request.campaign_id,
        state_id: request.state_id,
        candidates: targets,
        reason_id: request.reason_id,
        retention_profile_id: Some(request.retention_profile_id),
        rank_order: request.rank_order,
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    };
    prune.prune_plan_id = crate::model::derive_prune_plan_id(&prune)?;
    debug_assert!(prune.candidates.is_sorted());
    debug_assert_eq!(prune.prune_plan_id.kind, crate::identity::IdKind::PrunePlan);
    Ok(Some(prune))
}

/// Returns the sorted prune targets of a declared retention profile for the current state.
// r[impl campaign.pruning.valid]
pub(crate) fn planned_targets(
    state: &crate::frontier::FrontierState,
    retention_profile_id: crate::identity::Id,
    rank_order: &[u32],
) -> crate::error::CampaignResult<alloc::vec::Vec<crate::identity::Id>> {
    retention_profile_id.require_kind(crate::identity::IdKind::RetentionProfile)?;
    validate_rank_order(state, rank_order)?;
    let outstanding = state.outstanding_candidate_ids();
    let mut kept_outstanding: u64 = 0;
    let mut ordered = alloc::vec::Vec::with_capacity(state.candidates.len());
    for candidate in &state.candidates {
        if !matches!(candidate.status, crate::model::CandidateStatus::Retained) {
            continue;
        }
        let candidate_id = candidate.candidate.candidate_id;
        if outstanding.binary_search(&candidate_id).is_ok() {
            kept_outstanding = kept_outstanding
                .checked_add(crate::model::UNIT_COST)
                .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
            continue;
        }
        let projected = match &candidate.latest_guidance {
            Some(guidance) => Some(project_ranks(&guidance.ranks, rank_order)?),
            None => None,
        };
        ordered.push((projected, candidate_id));
    }
    ordered.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let first_pruned = first_pruned_position(state, kept_outstanding, ordered.len())?;
    let mut targets: alloc::vec::Vec<crate::identity::Id> = ordered
        .get(first_pruned..)
        .unwrap_or_default()
        .iter()
        .map(|(_, candidate_id)| *candidate_id)
        .collect();
    targets.sort();
    debug_assert!(targets.len() <= ordered.len());
    debug_assert!(crate::identity::unique_sorted_ids(&targets));
    Ok(targets)
}

/// Returns the position in the declared order where the pruned tail of the excess begins.
fn first_pruned_position(
    state: &crate::frontier::FrontierState,
    kept_outstanding: u64,
    ordered_len: usize,
) -> crate::error::CampaignResult<usize> {
    let bounds = &state.admission.bounds;
    let ordered_count = crate::identity::usize_to_u64(ordered_len)?;
    let free_slots = bounds
        .max_retained_candidates
        .saturating_sub(kept_outstanding);
    let excess = ordered_count
        .checked_sub(free_slots.min(ordered_count))
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    let pruned = excess.min(bounds.max_prune_batch);
    let first = ordered_count
        .checked_sub(pruned)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert!(pruned <= excess);
    debug_assert!(first <= ordered_count);
    usize::try_from(first).map_err(|_| crate::error::CampaignError::ArithmeticOverflow)
}

// r[impl campaign.guidance.declared_order]
fn validate_rank_order(
    state: &crate::frontier::FrontierState,
    rank_order: &[u32],
) -> crate::error::CampaignResult<()> {
    if rank_order.is_empty() {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    let mut sorted = rank_order.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != rank_order.len() {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    crate::policy::validation::validate_declared_rank_dimensions(state, rank_order)?;
    debug_assert!(!rank_order.is_empty());
    Ok(())
}

fn project_ranks(
    ranks: &[i64],
    rank_order: &[u32],
) -> crate::error::CampaignResult<alloc::vec::Vec<i64>> {
    let mut projected = alloc::vec::Vec::with_capacity(rank_order.len());
    for dimension in rank_order {
        let position = usize::try_from(*dimension)
            .map_err(|_| crate::error::CampaignError::ArithmeticOverflow)?;
        let rank = ranks
            .get(position)
            .copied()
            .ok_or(crate::error::CampaignError::InvalidGuidanceShape)?;
        projected.push(rank);
    }
    debug_assert_eq!(projected.len(), rank_order.len());
    Ok(projected)
}
