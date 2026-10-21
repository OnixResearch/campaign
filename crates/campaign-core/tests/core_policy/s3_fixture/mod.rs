//! Shared fixtures for settlement, retention, retirement, and stop controls.

pub(crate) type History = Vec<campaign_core::CampaignControlEvent>;

pub(crate) const RESULT_PAYLOAD: &[u8] = b"settlement-result-payload";

pub(crate) fn err<E: core::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

/// Returns the typed denial of an operation that must fail.
pub(crate) fn denial<T: core::fmt::Debug>(
    result: campaign_core::CampaignResult<T>,
) -> Result<campaign_core::CampaignError, String> {
    match result {
        Ok(value) => Err(format!("expected a typed denial, got {value:?}")),
        Err(error) => Ok(error),
    }
}

pub(crate) fn admitted_with(
    configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
) -> Result<(History, campaign_core::FrontierState), String> {
    let mut input = crate::suite::admission::default_input()?;
    configure(&mut input.bounds);
    input.bounds = input.bounds.derive().map_err(err)?;
    crate::suite::admission::admitted(input)
}

/// Appends one admitted event and rebuilds the frontier from the full history.
pub(crate) fn append(
    mut events: History,
    event: campaign_core::CampaignControlEvent,
) -> Result<(History, campaign_core::FrontierState), String> {
    events.push(event);
    let state = campaign_core::reconstruct(&events).map_err(err)?;
    Ok((events, state))
}

/// Returns the typed replay denial of one appended event.
pub(crate) fn replay_denial(
    events: &History,
    event: campaign_core::CampaignControlEvent,
) -> Result<campaign_core::CampaignError, String> {
    let mut forged = events.clone();
    forged.push(event);
    denial(campaign_core::reconstruct(&forged))
}

/// Asserts that a denial left the replayed frontier identity unchanged.
pub(crate) fn assert_unchanged(
    events: &History,
    state: &campaign_core::FrontierState,
) -> Result<(), String> {
    let rebuilt = campaign_core::reconstruct(events).map_err(err)?;
    assert_eq!(rebuilt.state_id, state.state_id);
    assert_eq!(&rebuilt, state);
    Ok(())
}

/// Admits and guides one candidate under the first root.
pub(crate) fn guided(
    events: History,
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    rank: i64,
) -> Result<(History, campaign_core::FrontierState, campaign_core::Id), String> {
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, state, label, crate::suite::constants::COST_ONE)?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        rank,
        crate::suite::constants::COST_ONE,
    )?;
    Ok((events, state, candidate_id))
}

/// Plans the next ranked selection and publishes it.
pub(crate) fn published(
    events: History,
    state: &campaign_core::FrontierState,
) -> Result<
    (
        History,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let selection = crate::suite::selection::plan(state)?;
    let (events, state) = crate::suite::selection::publish(events, selection.clone())?;
    Ok((events, state, selection))
}

/// Admits one guided candidate and publishes one selection of it.
pub(crate) fn selected(
    events: History,
    state: &campaign_core::FrontierState,
    label: &'static [u8],
) -> Result<
    (
        History,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let (events, state, _candidate_id) =
        guided(events, state, label, crate::suite::constants::RANK_HIGH)?;
    published(events, &state)
}

pub(crate) fn child(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    moment_label: &'static [u8],
    payload_label: &'static [u8],
) -> Result<campaign_core::ChildObservation, String> {
    campaign_core::ChildObservation {
        reservation_id: campaign_core::Id::absent(),
        child_moment_id: crate::suite::admission::xid(
            campaign_core::IdKind::ChildMoment,
            moment_label,
        ),
        discovery_id: campaign_core::Id::absent(),
        event_payload_id: crate::suite::admission::xid(
            campaign_core::IdKind::EventPayload,
            payload_label,
        ),
    }
    .derive(state.admission.campaign_id, selection.selection_id)
    .map_err(err)
}

pub(crate) fn observation(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    children: Vec<campaign_core::ChildObservation>,
) -> campaign_core::ResultObservation {
    campaign_core::ResultObservation {
        result_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        parent_moment_id: selection.parent_moment_id,
        operation_id: selection.operation_id,
        adapter_id: state.admission.adapter_id,
        payload_id: result_payload(selection),
        children,
        findings: Vec::new(),
        derived_records: Vec::new(),
    }
}

/// Result payload identity unique to one selection, so every accepted result payload is fresh.
pub(crate) fn result_payload(selection: &campaign_core::SelectionDecision) -> campaign_core::Id {
    campaign_core::identity::derive_id_with(
        campaign_core::IdKind::EventPayload,
        RESULT_PAYLOAD,
        |writer| Ok(writer.id(selection.selection_id)),
    )
    .unwrap_or_else(|_| campaign_core::Id::absent())
}

pub(crate) fn finding(label: &'static [u8]) -> campaign_core::FindingRef {
    campaign_core::FindingRef {
        finding_id: crate::suite::admission::xid(campaign_core::IdKind::Finding, label),
        priority_facts_id: crate::suite::admission::xid(campaign_core::IdKind::SourceFacts, label),
    }
}

/// Builds a result event that skips entry-point admission, as a forged history would.
pub(crate) fn forged_result(
    state: &campaign_core::FrontierState,
    observation: campaign_core::ResultObservation,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let observation = observation.derive().map_err(err)?;
    let event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::ResultObserved,
        state.admission.campaign_id,
        state.state_id,
        &[observation.result_id],
    )
    .map_err(err)?;
    Ok(campaign_core::CampaignControlEvent::ResultObserved(
        campaign_core::ResultEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            observation,
        },
    ))
}

pub(crate) fn closure(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    reason_label: &'static [u8],
    acknowledged: bool,
) -> Result<campaign_core::UnobservedClosure, String> {
    let mut closure = campaign_core::UnobservedClosure {
        closure_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, reason_label),
        uncertainty_acknowledged: acknowledged,
    };
    closure.closure_id = campaign_core::derive_closure_id(&closure).map_err(err)?;
    Ok(closure)
}

/// Builds a closure event that skips entry-point admission, as a forged history would.
pub(crate) fn forged_closure(
    state: &campaign_core::FrontierState,
    closure: campaign_core::UnobservedClosure,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::UnobservedClosed,
        state.admission.campaign_id,
        state.state_id,
        &[closure.closure_id, closure.selection_id, closure.reason_id],
    )
    .map_err(err)?;
    Ok(campaign_core::CampaignControlEvent::UnobservedClosed(
        campaign_core::ClosureEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            closure,
        },
    ))
}

/// Re-derives an earlier planned selection against a later state, as a replayed history would.
pub(crate) fn replanned_on(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut decision = selection.clone();
    decision.state_before_id = state.state_id;
    decision.publication_plan.state_before_id = state.state_id;
    decision.ordinal = state
        .selection_count
        .checked_add(crate::suite::constants::COST_ONE)
        .ok_or_else(|| String::from("ordinal overflow"))?;
    let decision = decision.derive().map_err(err)?;
    crate::suite::selection::event(decision)
}

pub(crate) fn retirement(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    origin: campaign_core::RetirementOrigin,
) -> campaign_core::RetirementRequest {
    campaign_core::RetirementRequest {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        policy_id: state.admission.policy_id,
        adapter_id: state.admission.adapter_id,
        candidate_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"retire"),
        origin,
    }
}

pub(crate) fn plan_decision(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::NextDecision, String> {
    Ok(campaign_core::plan_next(
        state,
        crate::suite::selection::request(state, crate::suite::selection::ranked_profile()?, None),
        &state.admission.projection,
    )
    .map_err(err)?
    .decision)
}

pub(crate) fn planned_stop(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::StopDecision, String> {
    match plan_decision(state)? {
        campaign_core::NextDecision::Stop(stop) => Ok(*stop),
        other => Err(format!("expected a stop, got {other:?}")),
    }
}
