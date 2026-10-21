/// Rejects every new-work control event after an accepted stop; results and closures stay admissible.
// r[impl campaign.stop.terminal]
pub(super) fn ensure_open_for_event(
    state: &crate::frontier::FrontierState,
    event: &crate::model::CampaignControlEvent,
) -> crate::error::CampaignResult<()> {
    let is_settlement = matches!(
        event,
        crate::model::CampaignControlEvent::ResultObserved(_)
            | crate::model::CampaignControlEvent::UnobservedClosed(_)
    );
    if state.stopped.is_some() && !is_settlement {
        return Err(crate::error::CampaignError::CampaignStopped);
    }
    Ok(())
}

// r[impl campaign.retirement]
pub(super) fn apply_retirement(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::RetirementEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if event.campaign_id != state.admission.campaign_id
        || event.policy_id != state.admission.policy_id
        || event.adapter_id != state.admission.adapter_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if crate::model::derive_retirement_invalidation_id(event)? != event.invalidation_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    crate::discipline::validate_dynamics(&event.dynamics())?;
    let expected = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidateRetired,
        event.campaign_id,
        event.state_before_id,
        &[event.candidate_id, event.reason_id, event.invalidation_id],
    )?;
    if expected != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let index = crate::frontier::validation::find_candidate_index(&state, event.candidate_id)?;
    debug_assert!(index < state.candidates.len());
    debug_assert_eq!(event.event_id, expected);
    if !matches!(
        state.candidates[index].status,
        crate::model::CandidateStatus::Retained
    ) {
        return Err(crate::error::CampaignError::InvalidRetirement);
    }
    state.candidates[index].status = crate::model::CandidateStatus::Retired {
        reason_id: event.reason_id,
    };
    Ok(state)
}

pub(super) fn apply_prune(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::PruneEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if event.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    event
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    crate::model::check_len_at_least_one(
        event.pruned_candidates.len(),
        state.admission.bounds.max_prune_batch,
        crate::error::CampaignError::UnchangedState,
    )?;
    let sorted = crate::identity::sorted_copy(&event.pruned_candidates);
    if !crate::identity::unique_sorted_ids(&sorted) {
        return Err(crate::error::CampaignError::DuplicateIdentity);
    }
    if sorted != event.pruned_candidates {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    check_prune_identities(event)?;
    crate::discipline::validate_dynamics(&event.dynamics())?;
    crate::policy::retention::check_prune_targets(
        &state,
        &sorted,
        event.retention_profile_id,
        &event.rank_order,
    )?;
    debug_assert!(!sorted.is_empty());
    debug_assert!(crate::identity::unique_sorted_ids(&sorted));
    for id in &sorted {
        let index = crate::frontier::validation::find_candidate_index(&state, *id)?;
        state.candidates[index].status = crate::model::CandidateStatus::Pruned {
            prune_plan_id: event.prune_plan_id,
        };
    }
    Ok(state)
}

fn check_prune_identities(event: &crate::model::PruneEvent) -> crate::error::CampaignResult<()> {
    let request = crate::model::PruneRequest {
        prune_plan_id: crate::identity::Id::absent(),
        campaign_id: event.campaign_id,
        state_id: event.state_before_id,
        candidates: event.pruned_candidates.clone(),
        reason_id: event.reason_id,
        retention_profile_id: event.retention_profile_id,
        rank_order: event.rank_order.clone(),
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    };
    if crate::model::derive_prune_plan_id(&request)? != event.prune_plan_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let invalidation_id =
        crate::model::derive_prune_invalidation_id(event.prune_plan_id, event.reason_id)?;
    if invalidation_id != event.invalidation_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let expected = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidatesPruned,
        event.campaign_id,
        event.state_before_id,
        &[event.prune_plan_id, event.invalidation_id],
    )?;
    if expected != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    debug_assert_eq!(invalidation_id.kind, crate::identity::IdKind::Invalidation);
    debug_assert_eq!(event.prune_plan_id.kind, crate::identity::IdKind::PrunePlan);
    Ok(())
}

pub(super) fn apply_stop(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::StopEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if state.stopped.is_some() {
        return Err(crate::error::CampaignError::CampaignStopped);
    }
    if event.campaign_id != state.admission.campaign_id
        || event.stop.campaign_id != state.admission.campaign_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if event.stop.state_before_id != event.state_before_id {
        return Err(crate::error::CampaignError::StaleState);
    }
    let expected_stop_id = crate::model::derive_stop_id(&event.stop)?;
    if expected_stop_id != event.stop.stop_id || !event.stop.incomplete_search_acknowledged {
        return Err(crate::error::CampaignError::InvalidStop);
    }
    crate::policy::retention::stopping::validate_stop_reason(&state, &event.stop.reason)?;
    let expected = crate::model::derive_event_id(
        &crate::model::ControlEventKind::StopAccepted,
        event.campaign_id,
        event.state_before_id,
        &[event.stop.stop_id],
    )?;
    if expected != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    debug_assert!(state.stopped.is_none());
    debug_assert!(event.stop.incomplete_search_acknowledged);
    state.stopped = Some(event.stop.clone());
    Ok(state)
}
