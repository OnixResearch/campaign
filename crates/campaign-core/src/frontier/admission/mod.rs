pub(super) fn apply_candidates(
    mut state: crate::frontier::FrontierState,
    batch: &crate::model::CandidateBatchEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if batch.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if batch.admission_policy_id != batch.admission_rule.admission_policy_id()? {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    batch
        .coalescing_policy_id
        .require_kind(crate::identity::IdKind::CoalescingPolicy)?;
    crate::model::check_len_at_least_one(
        batch.outcomes.len(),
        state.admission.bounds.max_candidate_batch,
        crate::error::CampaignError::MissingCandidate,
    )?;
    // The admitted records must match the bounded outcome list one to one; comparing the counts
    // first keeps an oversized forged record list from being validated entry by entry.
    let admitted_outcomes = batch
        .outcomes
        .iter()
        .filter(|outcome| outcome.outcome == crate::model::AdmissionDecision::Admitted)
        .count();
    if batch.candidates.len() != admitted_outcomes {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    for candidate in &batch.candidates {
        crate::frontier::validation::validate_candidate_admission(&state, candidate)?;
    }
    crate::frontier::admission::check_batch_outcomes(&state, batch)?;
    if batch.event_id != batch.expected_event_id()? {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let before = state.candidates.len();
    debug_assert!(batch.candidates.len() <= batch.outcomes.len());
    for candidate in &batch.candidates {
        state.candidates.push(crate::model::CandidateState {
            candidate: candidate.clone(),
            status: crate::model::CandidateStatus::Retained,
            selection_count: crate::model::NO_SELECTIONS,
            latest_guidance: None,
        });
    }
    crate::model::check_len(
        state.candidates.len(),
        state.admission.bounds.max_candidates,
    )?;
    debug_assert_eq!(
        state.candidates.len(),
        before.saturating_add(batch.candidates.len())
    );
    Ok(state)
}
// r[impl campaign.candidate_admission.coalesce]
fn check_batch_outcomes(
    state: &crate::frontier::FrontierState,
    batch: &crate::model::CandidateBatchEvent,
) -> crate::error::CampaignResult<()> {
    let submitted: alloc::vec::Vec<crate::identity::Id> = batch
        .outcomes
        .iter()
        .map(|outcome| outcome.candidate_id)
        .collect();
    let expected =
        crate::frontier::validation::candidate_outcomes(state, &submitted, batch.admission_rule)?;
    if expected != batch.outcomes {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let admitted = expected
        .iter()
        .filter(|outcome| outcome.outcome == crate::model::AdmissionDecision::Admitted)
        .map(|outcome| outcome.candidate_id);
    if !admitted.eq(batch
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id))
    {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    if batch.admission_outcomes_id != crate::model::derive_admission_outcomes_id(&batch.outcomes)? {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    debug_assert!(batch.candidates.len() <= batch.outcomes.len());
    Ok(())
}
// r[impl campaign.guidance.update]
// r[impl campaign.guidance.coalesced]
pub(super) fn apply_guidance(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::GuidanceEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if event.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let candidate_index =
        crate::frontier::validation::validate_guidance_shape(&state, &event.guidance)?;
    let previous = state.candidates[candidate_index].latest_guidance.clone();
    let expected = match &event.dynamics {
        crate::model::GuidanceDynamics::Revision {
            invalidated_guidance_id,
            reason_id,
            ..
        } => crate::model::GuidanceDynamics::revision(
            previous.as_ref(),
            &event.guidance,
            *invalidated_guidance_id,
            *reason_id,
        )?,
        _ => crate::model::GuidanceDynamics::relaxation(previous.as_ref(), &event.guidance)?,
    };
    if expected != event.dynamics {
        return Err(crate::error::CampaignError::StaleGuidance);
    }
    if let Some(dynamics) = event
        .dynamics
        .frontier_dynamics(previous.as_ref(), &event.guidance)?
    {
        crate::discipline::validate_dynamics(&dynamics)?;
    }
    if event.admission_outcomes_id != event.dynamics.outcomes_id(event.guidance.guidance_id)?
        || event.event_id != event.expected_event_id()?
    {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    if event.dynamics.adds_record() {
        state.guidance_record_count =
            crate::frontier::validation::next_guidance_record_count(&state)?;
        state.candidates[candidate_index].latest_guidance = Some(event.guidance.clone());
    }
    debug_assert!(candidate_index < state.candidates.len());
    Ok(state)
}
