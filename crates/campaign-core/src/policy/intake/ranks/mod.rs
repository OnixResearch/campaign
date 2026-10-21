/// Public Campaign core API operation: admits one relaxation-only guidance update: discovery, strict improvement, or coalesced duplicate.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
// r[impl campaign.guidance]
pub fn admit_guidance(
    state: &crate::frontier::FrontierState,
    guidance: crate::model::GuidanceRecord,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    let guidance = guidance.derive()?;
    let index = crate::frontier::validation::validate_guidance_shape(state, &guidance)?;
    let dynamics = crate::model::GuidanceDynamics::relaxation(
        state.candidates[index].latest_guidance.as_ref(),
        &guidance,
    )?;
    record_event(state, guidance, dynamics)
}
/// Public Campaign core API operation: admits an explicit revision that invalidates the latest guidance record of one candidate.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
// r[impl campaign.guidance.revision]
pub fn revise_guidance(
    state: &crate::frontier::FrontierState,
    request: crate::model::GuidanceRevisionRequest,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    let successor = request.successor.derive()?;
    if request.candidate_id != successor.candidate_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let index = crate::frontier::validation::validate_guidance_shape(state, &successor)?;
    let dynamics = crate::model::GuidanceDynamics::revision(
        state.candidates[index].latest_guidance.as_ref(),
        &successor,
        request.invalidated_guidance_id,
        request.reason_id,
    )?;
    record_event(state, successor, dynamics)
}
// r[impl campaign.guidance.invalid]
// r[impl campaign.guidance.coalesced]
fn record_event(
    state: &crate::frontier::FrontierState,
    guidance: crate::model::GuidanceRecord,
    dynamics: crate::model::GuidanceDynamics,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    if dynamics.adds_record() {
        crate::frontier::validation::next_guidance_record_count(state)?;
    }
    let mut event = crate::model::GuidanceEvent {
        event_id: crate::identity::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        admission_outcomes_id: dynamics.outcomes_id(guidance.guidance_id)?,
        guidance,
        dynamics,
    };
    event.event_id = event.expected_event_id()?;
    debug_assert_eq!(event.guidance.campaign_id, state.admission.campaign_id);
    Ok(crate::model::CampaignControlEvent::GuidanceAdmitted(event))
}
