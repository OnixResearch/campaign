//! Closeout-audit regressions: replayed selection policy, publication-plan pinning, payload
//! uniqueness, genuine receipts, canonical state, re-planned scheduling explanations,
//! control-event headroom, and budget arithmetic.

mod headroom;
mod receipts;
mod replay;
mod scheduling;
mod settlement;

pub(super) fn guided_pair() -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
        campaign_core::Id,
        campaign_core::Id,
    ),
    String,
> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, best) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"audit-best",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, lower) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"audit-lower",
        crate::suite::constants::RANK_LOW,
    )?;
    Ok((events, state, best, lower))
}

/// Builds a selection of `candidate_id` directly from the state, as a forged history would.
pub(super) fn selection_of(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    profile: &campaign_core::SelectionProfile,
) -> Result<campaign_core::SelectionDecision, String> {
    let candidate = state
        .candidate(candidate_id)
        .map_err(crate::suite::s3_fixture::err)?;
    let guidance = candidate
        .latest_guidance
        .as_ref()
        .ok_or_else(|| String::from("guided"))?;
    assert_eq!(candidate.candidate.candidate_id, candidate_id);
    assert_eq!(guidance.candidate_id, candidate_id);
    let projection = &state.admission.projection;
    campaign_core::SelectionDecision {
        selection_id: campaign_core::Id::absent(),
        ordinal: state.selection_count.saturating_add(1),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidate_id,
        parent_moment_id: candidate.candidate.parent_moment_id,
        operation_id: campaign_core::Id::absent(),
        guidance_id: guidance.guidance_id,
        profile_id: profile.profile_id,
        entropy_id: None,
        reserved_costs: candidate.candidate.declared_costs.clone(),
        publication_plan: campaign_core::PublicationPlan {
            event_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            control_branch_id: projection.control_branch_id,
            expected_head_id: projection.head_id,
            expected_generation: projection.generation,
            state_before_id: state.state_id,
            selection_id: campaign_core::Id::absent(),
        },
    }
    .derive()
    .map_err(crate::suite::s3_fixture::err)
}
