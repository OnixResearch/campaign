pub(crate) fn profile() -> Result<campaign_core::SelectionProfile, String> {
    campaign_core::SelectionProfile {
        profile_id: campaign_core::Id::absent(),
        scheduling_policy_id: crate::suite::admission::tid(
            campaign_core::IdKind::SchedulingProfile,
            b"adversarial-scheduling",
        ),
        admission_policy_id: crate::suite::admission::tid(
            campaign_core::IdKind::AdmissionPolicy,
            b"adversarial-admission-policy",
        ),
        coalescing_policy_id: crate::suite::admission::tid(
            campaign_core::IdKind::CoalescingPolicy,
            b"adversarial-coalescing-policy",
        ),
        mode: campaign_core::SelectionMode::Ranked,
        decay: campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: crate::suite::constants::COST_ONE,
        },
        stopping: campaign_core::StoppingRule {
            finding_limit: None,
        },
        implementation: campaign_core::PolicyImplementation::ProductionCompiled {
            policy_id: crate::suite::admission::tid(
                campaign_core::IdKind::Policy,
                b"adversarial-policy",
            ),
        },
    }
    .derive()
    .map_err(|error| format!("{error:?}"))
}
pub(crate) fn request(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionRequest, String> {
    Ok(campaign_core::SelectionRequest {
        request_id: crate::suite::admission::tid(
            campaign_core::IdKind::Request,
            b"adversarial-request",
        ),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile: profile()?,
        entropy: None,
        expected_generation: state.admission.projection.generation,
        expected_head_id: state.admission.projection.head_id,
    })
}
pub(crate) fn plan(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionDecision, String> {
    let decision = campaign_core::plan_next(state, request(state)?, &state.admission.projection)
        .map_err(|error| format!("{error:?}"))?
        .decision;
    let campaign_core::NextDecision::Select(selection) = decision else {
        return Err(format!("expected selection, got {decision:?}"));
    };
    Ok(*selection)
}
pub(crate) fn selected_state(
    resource_budget: u64,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let (events, state) =
        crate::suite::admission::admitted_state(crate::suite::admission::Limits {
            resource_budget,
            max_findings: crate::suite::constants::BOUND_ONE,
        })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"adversarial-candidate",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state) = crate::suite::candidate::guide(events, &state, candidate_id)?;
    let selection = plan(&state)?;
    let mut selected_events = events;
    selected_events.push(published(selection.clone())?);
    let selected =
        (campaign_core::reconstruct(&selected_events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(selected.admission.campaign_id, state.admission.campaign_id);
    assert!(
        selected
            .outstanding
            .iter()
            .any(|record| record.selection_id == selection.selection_id)
    );
    Ok((selected_events, selected, selection))
}
/// Publication event of a selection planned with the adversarial profile and no entropy.
pub(crate) fn published(
    selection: campaign_core::SelectionDecision,
) -> Result<campaign_core::CampaignControlEvent, String> {
    campaign_core::SelectionEvent::new(selection, profile()?, None)
        .map(campaign_core::CampaignControlEvent::SelectionPublished)
        .map_err(|error| format!("{error:?}"))
}
