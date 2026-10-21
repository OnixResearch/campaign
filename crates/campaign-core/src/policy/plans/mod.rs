// r[impl campaign.selection]
pub(super) fn build_selection(
    state: &crate::frontier::FrontierState,
    inputs: &crate::policy::decision::PlanningInputs<'_>,
    candidate: &crate::model::CandidateState,
    guidance: &crate::model::GuidanceRecord,
) -> crate::error::CampaignResult<crate::model::SelectionDecision> {
    let ordinal = state
        .selection_count
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    let entropy_id = inputs.entropy.map(|entropy| entropy.entropy_id);
    let decision = crate::model::SelectionDecision {
        selection_id: crate::identity::Id::absent(),
        ordinal,
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidate_id: candidate.candidate.candidate_id,
        parent_moment_id: candidate.candidate.parent_moment_id,
        operation_id: crate::identity::Id::absent(),
        guidance_id: guidance.guidance_id,
        profile_id: inputs.profile.profile_id,
        entropy_id,
        reserved_costs: candidate.candidate.declared_costs.clone(),
        publication_plan: crate::model::PublicationPlan {
            event_id: crate::identity::Id::absent(),
            campaign_id: state.admission.campaign_id,
            control_branch_id: inputs.control_branch_id,
            expected_head_id: inputs.head_id,
            expected_generation: inputs.generation,
            state_before_id: state.state_id,
            selection_id: crate::identity::Id::absent(),
        },
    };
    let derived = decision.derive()?;
    debug_assert!(derived.ordinal > state.selection_count);
    debug_assert_eq!(derived.publication_plan.selection_id, derived.selection_id);
    Ok(derived)
}
// r[impl campaign.stop.non_claim]
pub(super) fn build_stop(
    state: &crate::frontier::FrontierState,
    reason: crate::model::StopReason,
) -> crate::error::CampaignResult<crate::model::StopDecision> {
    let mut stop = crate::model::StopDecision {
        stop_id: crate::identity::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        reason,
        incomplete_search_acknowledged: true,
    };
    stop.stop_id = crate::model::derive_stop_id(&stop)?;
    Ok(stop)
}
pub(super) fn decision_plan(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
    decision: crate::model::NextDecision,
    work: crate::model::PlanningWork,
) -> crate::model::DecisionPlan {
    crate::model::DecisionPlan {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        decision,
        scheduling_policy_id: profile.scheduling_policy_id,
        admission_policy_id: profile.admission_policy_id,
        coalescing_policy_id: profile.coalescing_policy_id,
        work,
    }
}
