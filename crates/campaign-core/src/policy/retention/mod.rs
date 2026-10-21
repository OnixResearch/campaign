pub(crate) mod planning;
pub(crate) mod stopping;

pub use planning::plan_pruning;
pub use stopping::admit_stop;

/// Admits an explicit retirement of one retained candidate against the exact current state.
///
/// # Errors
///
/// Returns `CampaignStopped` after an accepted stop, `CrossedIdentity` for another campaign, policy,
/// or adapter, `StaleState` for another state, `MissingCandidate` for an unknown candidate, and
/// `InvalidRetirement` for a candidate that is no longer retained.
// r[impl campaign.retirement]
pub fn retire_candidate(
    state: &crate::frontier::FrontierState,
    request: crate::model::RetirementRequest,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    if request.campaign_id != state.admission.campaign_id
        || request.policy_id != state.admission.policy_id
        || request.adapter_id != state.admission.adapter_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if request.state_id != state.state_id {
        return Err(crate::error::CampaignError::StaleState);
    }
    request
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    let candidate = state.candidate(request.candidate_id)?;
    if !matches!(candidate.status, crate::model::CandidateStatus::Retained) {
        return Err(crate::error::CampaignError::InvalidRetirement);
    }
    debug_assert_eq!(candidate.candidate.candidate_id, request.candidate_id);
    debug_assert_eq!(request.state_id, state.state_id);
    let mut event = crate::model::RetirementEvent {
        event_id: crate::identity::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidate_id: request.candidate_id,
        reason_id: request.reason_id,
        policy_id: request.policy_id,
        adapter_id: request.adapter_id,
        origin: request.origin,
        invalidation_id: crate::identity::Id::absent(),
    };
    event.invalidation_id = crate::model::derive_retirement_invalidation_id(&event)?;
    event.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidateRetired,
        event.campaign_id,
        event.state_before_id,
        &[event.candidate_id, event.reason_id, event.invalidation_id],
    )?;
    Ok(crate::model::CampaignControlEvent::CandidateRetired(event))
}

/// Admits an explicit prune of retained candidates without outstanding selections.
///
/// A request that carries a retention profile must name exactly the targets of `plan_pruning`
/// for that profile; a request without a profile is an explicit caller list.
///
/// # Errors
///
/// Returns `InvalidPruning` for deletion or rewrite claims, outstanding or non-retained targets,
/// and targets that differ from the declared retention plan; `StaleState` for another state.
// r[impl campaign.pruning]
// r[impl campaign.guidance.declared_order]
pub fn prune_frontier(
    state: &crate::frontier::FrontierState,
    mut request: crate::model::PruneRequest,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    if request.campaign_id != state.admission.campaign_id || request.state_id != state.state_id {
        return Err(crate::error::CampaignError::StaleState);
    }
    if request.claims_artifact_deletion
        || request.claims_history_rewrite
        || request.claims_safe_deletion
    {
        return Err(crate::error::CampaignError::InvalidPruning);
    }
    request
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    request.candidates.sort();
    crate::model::check_len_at_least_one(
        request.candidates.len(),
        state.admission.bounds.max_prune_batch,
        crate::error::CampaignError::UnchangedState,
    )?;
    if !crate::identity::unique_sorted_ids(&request.candidates) {
        return Err(crate::error::CampaignError::DuplicateIdentity);
    }
    debug_assert!(!request.candidates.is_empty());
    debug_assert!(crate::identity::unique_sorted_ids(&request.candidates));
    crate::policy::retention::check_prune_targets(
        state,
        &request.candidates,
        request.retention_profile_id,
        &request.rank_order,
    )?;
    request.prune_plan_id = crate::model::derive_prune_plan_id(&request)?;
    let invalidation_id =
        crate::model::derive_prune_invalidation_id(request.prune_plan_id, request.reason_id)?;
    let event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidatesPruned,
        state.admission.campaign_id,
        state.state_id,
        &[request.prune_plan_id, invalidation_id],
    )?;
    Ok(crate::model::CampaignControlEvent::CandidatesPruned(
        crate::model::PruneEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            pruned_candidates: request.candidates,
            prune_plan_id: request.prune_plan_id,
            reason_id: request.reason_id,
            retention_profile_id: request.retention_profile_id,
            rank_order: request.rank_order,
            invalidation_id,
        },
    ))
}

/// Checks that every target is a known retained candidate without outstanding work and, when a
/// retention profile is declared, that the sorted targets equal its deterministic plan.
// r[impl campaign.pruning]
pub(crate) fn check_prune_targets(
    state: &crate::frontier::FrontierState,
    sorted_targets: &[crate::identity::Id],
    retention_profile_id: Option<crate::identity::Id>,
    rank_order: &[u32],
) -> crate::error::CampaignResult<()> {
    let outstanding = state.outstanding_candidate_ids();
    for candidate_id in sorted_targets {
        let candidate = state.candidate(*candidate_id)?;
        if outstanding.binary_search(candidate_id).is_ok()
            || !matches!(candidate.status, crate::model::CandidateStatus::Retained)
        {
            return Err(crate::error::CampaignError::InvalidPruning);
        }
    }
    let Some(profile_id) = retention_profile_id else {
        if !rank_order.is_empty() {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        return Ok(());
    };
    let planned =
        crate::policy::retention::planning::planned_targets(state, profile_id, rank_order)?;
    if planned.as_slice() != sorted_targets {
        return Err(crate::error::CampaignError::InvalidPruning);
    }
    debug_assert!(planned.is_sorted());
    Ok(())
}
