//! Shared worlds: a populated frontier (one candidate selected, settled with two children, and
//! selected again) and planned decisions over it.

pub(crate) struct Populated {
    pub(crate) events: Vec<campaign_core::CampaignControlEvent>,
    pub(crate) settled_state: campaign_core::FrontierState,
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) candidate_id: campaign_core::Id,
    pub(crate) first_selection: campaign_core::SelectionDecision,
    pub(crate) second_selection: campaign_core::SelectionDecision,
    pub(crate) result: campaign_core::ResultObservation,
}

pub(crate) fn observed<T>(value: campaign_core::CampaignResult<T>) -> Result<T, String> {
    value.map_err(|error| format!("{error:?}"))
}

pub(crate) fn denial<T: std::fmt::Debug>(
    value: campaign_core::CampaignResult<T>,
) -> Result<campaign_core::CampaignError, String> {
    match value {
        Ok(value) => Err(format!("expected a typed denial, got {value:?}")),
        Err(error) => Ok(error),
    }
}

fn two_child_result(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
) -> Result<campaign_core::ResultObservation, String> {
    let mut observation = crate::suite::observation::result_for(
        state,
        selection,
        b"populated-child-a",
        b"populated-payload-a",
    )?;
    let sibling = observed(
        campaign_core::ChildObservation {
            reservation_id: campaign_core::Id::absent(),
            child_moment_id: crate::suite::admission::xid(
                campaign_core::IdKind::ChildMoment,
                b"populated-child-b",
            ),
            discovery_id: campaign_core::Id::absent(),
            event_payload_id: crate::suite::admission::xid(
                campaign_core::IdKind::EventPayload,
                b"populated-payload-b",
            ),
        }
        .derive(state.admission.campaign_id, selection.selection_id),
    )?;
    observation.children.push(sibling);
    assert_eq!(observation.children.len(), 2);
    assert_eq!(observation.selection_id, selection.selection_id);
    Ok(observation)
}

pub(crate) fn populated() -> Result<Populated, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"populated",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let first_selection = crate::suite::selection::plan(&state)?;
    let (mut events, state) = crate::suite::selection::publish(events, first_selection.clone())?;
    let result_event = observed(campaign_core::admit_result(
        &state,
        two_child_result(&state, &first_selection)?,
    ))?;
    let campaign_core::CampaignControlEvent::ResultObserved(result_event_value) = &result_event
    else {
        return Err(String::from("result admission produced another event"));
    };
    let result = result_event_value.observation.clone();
    events.push(result_event);
    let settled_state = observed(campaign_core::reconstruct(&events))?;
    let second_selection = crate::suite::selection::plan(&settled_state)?;
    let (events, state) = crate::suite::selection::publish(events, second_selection.clone())?;
    assert_eq!(first_selection.candidate_id, second_selection.candidate_id);
    assert_eq!(state.outstanding.len(), 1);
    assert_eq!(state.accepted_results, vec![result.result_id]);
    Ok(Populated {
        events,
        settled_state,
        state,
        candidate_id,
        first_selection,
        second_selection,
        result,
    })
}

pub(crate) struct Planned {
    pub(crate) state: campaign_core::FrontierState,
    /// The state's current history binding the decision was planned against.
    pub(crate) binding: campaign_core::ProjectionBinding,
    pub(crate) request: campaign_core::SelectionRequest,
    pub(crate) plan: campaign_core::DecisionPlan,
}

pub(crate) fn planned(state: campaign_core::FrontierState) -> Result<Planned, String> {
    let binding = crate::suite::admission::current_projection(&state)?;
    let request = crate::suite::selection::request_on(
        &state,
        crate::suite::selection::ranked_profile()?,
        None,
        &binding,
    );
    let plan = observed(campaign_core::plan_next(&state, request.clone(), &binding))?;
    Ok(Planned {
        state,
        binding,
        request,
        plan,
    })
}

pub(crate) fn project(
    planned: &Planned,
) -> campaign_core::CampaignResult<campaign_core::CampaignReceipt> {
    campaign_core::project_receipt(
        &planned.state,
        &planned.plan,
        &planned.request,
        &planned.binding,
    )
}

/// A plan that selects the populated candidate again from the settled state.
pub(crate) fn selecting() -> Result<Planned, String> {
    planned(populated()?.settled_state)
}
