//! Canonical forms of settlement and retention events: results, closures, retirements, prunes,
//! and stops.

/// Re-derives every child's reservation and discovery identity and the result identity, with
/// children, findings and derived records in canonical order.
pub(super) fn result(
    event: &crate::model::ResultEvent,
) -> crate::error::CampaignResult<crate::model::ResultEvent> {
    let mut observation = event.observation.clone();
    observation.children = event
        .observation
        .children
        .iter()
        .map(|child| {
            child.clone().derive(
                event.observation.campaign_id,
                event.observation.selection_id,
            )
        })
        .collect::<crate::error::CampaignResult<alloc::vec::Vec<_>>>()?;
    let mut derived = event.clone();
    derived.observation = observation.derive()?;
    derived.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::ResultObserved,
        derived.campaign_id,
        derived.state_before_id,
        &[derived.observation.result_id],
    )?;
    debug_assert_eq!(
        derived.observation.children.len(),
        event.observation.children.len()
    );
    Ok(derived)
}

/// Re-derives the closure identity.
pub(super) fn closure(
    event: &crate::model::ClosureEvent,
) -> crate::error::CampaignResult<crate::model::ClosureEvent> {
    let mut derived = event.clone();
    derived.closure.closure_id = crate::model::derive_closure_id(&event.closure)?;
    derived.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::UnobservedClosed,
        derived.campaign_id,
        derived.state_before_id,
        &[
            derived.closure.closure_id,
            derived.closure.selection_id,
            derived.closure.reason_id,
        ],
    )?;
    Ok(derived)
}

/// Re-derives the retirement's invalidation identity.
pub(super) fn retirement(
    event: &crate::model::RetirementEvent,
) -> crate::error::CampaignResult<crate::model::RetirementEvent> {
    let mut derived = event.clone();
    derived.invalidation_id = crate::model::derive_retirement_invalidation_id(event)?;
    derived.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidateRetired,
        derived.campaign_id,
        derived.state_before_id,
        &[
            derived.candidate_id,
            derived.reason_id,
            derived.invalidation_id,
        ],
    )?;
    Ok(derived)
}

/// Re-derives the prune plan and invalidation identities over canonically sorted targets.
pub(super) fn prune(
    event: &crate::model::PruneEvent,
) -> crate::error::CampaignResult<crate::model::PruneEvent> {
    let mut derived = event.clone();
    derived.pruned_candidates.sort();
    if !crate::identity::unique_sorted_ids(&derived.pruned_candidates) {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    derived.prune_plan_id = crate::model::derive_prune_plan_id(&crate::model::PruneRequest {
        prune_plan_id: crate::identity::Id::absent(),
        campaign_id: derived.campaign_id,
        state_id: derived.state_before_id,
        candidates: derived.pruned_candidates.clone(),
        reason_id: derived.reason_id,
        retention_profile_id: derived.retention_profile_id,
        rank_order: derived.rank_order.clone(),
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    })?;
    derived.invalidation_id =
        crate::model::derive_prune_invalidation_id(derived.prune_plan_id, derived.reason_id)?;
    derived.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::CandidatesPruned,
        derived.campaign_id,
        derived.state_before_id,
        &[derived.prune_plan_id, derived.invalidation_id],
    )?;
    debug_assert_eq!(
        derived.prune_plan_id.kind,
        crate::identity::IdKind::PrunePlan
    );
    debug_assert_eq!(
        derived.invalidation_id.kind,
        crate::identity::IdKind::Invalidation
    );
    Ok(derived)
}

/// Re-derives the stop identity with a budget stop's dimensions in canonical order.
pub(super) fn stop(
    event: &crate::model::StopEvent,
) -> crate::error::CampaignResult<crate::model::StopEvent> {
    let mut derived = event.clone();
    if let crate::model::StopReason::BudgetExhausted { dimensions } = &mut derived.stop.reason {
        dimensions.sort();
    }
    derived.stop.stop_id = crate::model::derive_stop_id(&derived.stop)?;
    derived.event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::StopAccepted,
        derived.campaign_id,
        derived.state_before_id,
        &[derived.stop.stop_id],
    )?;
    Ok(derived)
}
