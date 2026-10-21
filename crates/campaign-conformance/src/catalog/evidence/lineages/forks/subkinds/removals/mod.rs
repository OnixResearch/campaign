//! Candidate heads that name an admitted candidate: a batch of a new operation with a coalesced
//! duplicate of the first admitted candidate, a consumer or policy retirement of the first
//! retained candidate, and an explicit prune of the first retained candidate without outstanding
//! selections (each first in suite operation order).

/// Handle of the new operation a coalesced-batch head admits (above every fill operation and the
/// batch head's).
const COALESCED_OPERATION: u32 = 201;

/// A batch of a new operation and a duplicate of the first admitted candidate, when the session
/// coalesces duplicates and `state` admits one more candidate in a batch of two.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the batch.
pub(super) fn coalesced_batch(
    session: &mut crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let bounds = &state.admission.bounds;
    let has_room = u64::try_from(state.candidates.len())
        .is_ok_and(|count| count < bounds.max_candidates)
        && bounds.max_candidate_batch >= 2;
    let is_coalescing = matches!(
        session.rule.duplicate_candidate,
        campaign_core::DuplicateCandidateRule::Coalesce
    );
    let Some((_, existing)) = super::first_in_suite_order(session, state, |_| true)? else {
        return Ok(None);
    };
    if !has_room || !is_coalescing {
        return Ok(None);
    }
    let duplicate = existing.candidate.clone();
    let mut records = session.candidates(&[crate::catalog::common::operation(
        crate::vocabulary::OperationHandle(COALESCED_OPERATION),
        crate::harness::history::ROOT,
        crate::catalog::common::UNIT_COST,
    )])?;
    records.push(duplicate);
    debug_assert_eq!(records.len(), 2);
    Ok(Some(campaign_core::admit_candidates(
        state,
        records,
        session.rule,
        crate::harness::records::coalescing_policy_id()?,
    )?))
}

/// The retirement (by consumer or by policy) or the explicit prune `kind` of the first fitting
/// retained candidate of `state`; `None` when none fits.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the removal.
pub(super) fn event(
    session: &mut crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
    kind: super::SubKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let is_prune = kind == super::SubKind::Prune;
    let chosen = super::first_in_suite_order(session, state, |candidate| {
        candidate.status == campaign_core::CandidateStatus::Retained
            && !(is_prune && state.has_outstanding_for_candidate(candidate.candidate.candidate_id))
    })?;
    let Some((_, candidate)) = chosen else {
        return Ok(None);
    };
    let candidate_id = candidate.candidate.candidate_id;
    if is_prune {
        let request = campaign_core::PruneRequest {
            prune_plan_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            candidates: alloc::vec![candidate_id],
            reason_id: session
                .harness
                .adapter
                .reason(crate::vocabulary::ScenarioReason::Pruning)?,
            retention_profile_id: None,
            rank_order: alloc::vec::Vec::new(),
            claims_artifact_deletion: false,
            claims_history_rewrite: false,
            claims_safe_deletion: false,
        };
        return Ok(Some(campaign_core::prune_frontier(state, request)?));
    }
    let origin = if kind == super::SubKind::PolicyRetirement {
        campaign_core::RetirementOrigin::Policy
    } else {
        campaign_core::RetirementOrigin::Consumer
    };
    debug_assert!(matches!(
        kind,
        super::SubKind::ConsumerRetirement | super::SubKind::PolicyRetirement
    ));
    let request = campaign_core::RetirementRequest {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        policy_id: state.admission.policy_id,
        adapter_id: state.admission.adapter_id,
        candidate_id,
        reason_id: session
            .harness
            .adapter
            .reason(crate::vocabulary::ScenarioReason::Retirement)?,
        origin,
    };
    Ok(Some(campaign_core::retire_candidate(state, request)?))
}
