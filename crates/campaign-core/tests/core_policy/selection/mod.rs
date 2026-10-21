pub(crate) fn ranked_profile() -> Result<campaign_core::SelectionProfile, String> {
    profile(campaign_core::SelectionMode::Ranked)
}
pub(crate) fn profile(
    mode: campaign_core::SelectionMode,
) -> Result<campaign_core::SelectionProfile, String> {
    campaign_core::SelectionProfile {
        profile_id: campaign_core::Id::absent(),
        scheduling_policy_id: crate::suite::admission::xid(
            campaign_core::IdKind::SchedulingProfile,
            b"schedule",
        ),
        admission_policy_id: crate::suite::admission::xid(
            campaign_core::IdKind::AdmissionPolicy,
            b"admit",
        ),
        coalescing_policy_id: crate::suite::admission::xid(
            campaign_core::IdKind::CoalescingPolicy,
            b"coalesce",
        ),
        mode,
        decay: campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: crate::suite::constants::COST_ONE,
        },
        stopping: campaign_core::StoppingRule {
            finding_limit: None,
        },
        implementation: campaign_core::PolicyImplementation::ProductionCompiled {
            policy_id: crate::suite::admission::xid(campaign_core::IdKind::Policy, b"policy"),
        },
    }
    .derive()
    .map_err(|error| format!("{error:?}"))
}
pub(crate) fn request(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> campaign_core::SelectionRequest {
    crate::suite::selection::request_on(state, profile, entropy, &state.admission.projection)
}
/// A request for `state` whose expected generation and head are those of `binding`.
pub(crate) fn request_on(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
    binding: &campaign_core::ProjectionBinding,
) -> campaign_core::SelectionRequest {
    campaign_core::SelectionRequest {
        request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"request"),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile,
        entropy,
        expected_generation: binding.generation,
        expected_head_id: binding.head_id,
    }
}
pub(crate) fn plan(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionDecision, String> {
    let decision = campaign_core::plan_next(
        state,
        request(state, ranked_profile()?, None),
        &state.admission.projection,
    )
    .map_err(|error| format!("{error:?}"))?
    .decision;
    let campaign_core::NextDecision::Select(selection) = decision else {
        return Err(format!("expected selection, got {decision:?}"));
    };
    Ok(*selection)
}
pub(crate) fn publish(
    mut events: Vec<campaign_core::CampaignControlEvent>,
    selection: campaign_core::SelectionDecision,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    events.push(event(selection)?);
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    Ok((events, state))
}
/// Publication event of a selection planned with the ranked profile and no entropy.
pub(crate) fn event(
    selection: campaign_core::SelectionDecision,
) -> Result<campaign_core::CampaignControlEvent, String> {
    event_with(selection, ranked_profile()?, None)
}
/// Publication event of a selection planned with `profile` and `entropy`.
pub(crate) fn event_with(
    selection: campaign_core::SelectionDecision,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> Result<campaign_core::CampaignControlEvent, String> {
    campaign_core::SelectionEvent::new(selection, profile, entropy)
        .map(campaign_core::CampaignControlEvent::SelectionPublished)
        .map_err(|error| format!("{error:?}"))
}
