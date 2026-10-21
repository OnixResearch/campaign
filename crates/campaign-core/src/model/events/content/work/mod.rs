//! Canonical forms of new-work events: candidate batches, guidance, and selections.

/// Re-derives every admitted candidate, the admission policy and the outcome list identity, with
/// candidates and outcomes in canonical identity order (an admitted outcome before the coalesced
/// duplicates of the same identity).
pub(super) fn candidates(
    batch: &crate::model::CandidateBatchEvent,
) -> crate::error::CampaignResult<crate::model::CandidateBatchEvent> {
    let mut derived = batch.clone();
    derived.candidates = batch
        .candidates
        .iter()
        .map(|candidate| candidate.clone().derive())
        .collect::<crate::error::CampaignResult<alloc::vec::Vec<_>>>()?;
    derived
        .candidates
        .sort_by_key(|candidate| candidate.candidate_id);
    derived.outcomes.sort_by_key(|outcome| {
        (
            outcome.candidate_id,
            !matches!(outcome.outcome, crate::model::AdmissionDecision::Admitted),
        )
    });
    derived.admission_policy_id = batch.admission_rule.admission_policy_id()?;
    derived.admission_outcomes_id = crate::model::derive_admission_outcomes_id(&derived.outcomes)?;
    derived.event_id = derived.expected_event_id()?;
    debug_assert_eq!(derived.candidates.len(), batch.candidates.len());
    Ok(derived)
}

/// Re-derives the guidance record, a revision's invalidation identity, and the outcome identity.
pub(super) fn guidance(
    event: &crate::model::GuidanceEvent,
) -> crate::error::CampaignResult<crate::model::GuidanceEvent> {
    let mut derived = event.clone();
    derived.guidance = event.guidance.clone().derive()?;
    if let crate::model::GuidanceDynamics::Revision {
        invalidated_guidance_id,
        reason_id,
        ..
    } = &event.dynamics
    {
        derived.dynamics = crate::model::GuidanceDynamics::Revision {
            invalidated_guidance_id: *invalidated_guidance_id,
            invalidation_id: crate::model::derive_revision_invalidation_id(
                *invalidated_guidance_id,
                *reason_id,
            )?,
            reason_id: *reason_id,
        };
    }
    derived.admission_outcomes_id = derived.dynamics.outcomes_id(derived.guidance.guidance_id)?;
    derived.event_id = derived.expected_event_id()?;
    debug_assert_eq!(derived.guidance.candidate_id, event.guidance.candidate_id);
    Ok(derived)
}

/// Re-derives the decision, profile and ticket; the event identity is the decision's publication
/// event identity.
pub(super) fn selection(
    event: &crate::model::SelectionEvent,
) -> crate::error::CampaignResult<crate::model::SelectionEvent> {
    let decision = event.decision.clone().derive()?;
    let derived = crate::model::SelectionEvent {
        event_id: decision.publication_plan.event_id,
        decision,
        profile: alloc::boxed::Box::new(event.profile.as_ref().clone().derive()?),
        entropy: event
            .entropy
            .as_ref()
            .map(|ticket| ticket.clone().derive())
            .transpose()?,
    };
    derived.check_policy_inputs()?;
    debug_assert_eq!(derived.event_id, derived.decision.publication_plan.event_id);
    Ok(derived)
}
