//! Corrupted planning and publication inputs: crossed or stale requests and projections,
//! crossed or misplaced entropy tickets, tampered states, and stale selection events.

/// Largest generated projection or request generation skew.
const MAX_SKEW: u64 = 5;

/// Request facts that disagree with the state or the current projection.
#[derive(Clone, Debug)]
pub(crate) enum RequestFault {
    CrossedCampaign,
    StaleState(u8),
    StaleGeneration(u64),
    AdvancedProjection(u64),
    StaleHead,
    CrossedRequestKind,
}

/// Entropy tickets that are missing, bound to another state, or offered to ranked mode.
#[derive(Clone, Debug)]
pub(crate) enum EntropyFault {
    Missing,
    CrossedState(crate::suite::requests::Draws),
    OfferedToRanked(crate::suite::requests::Draws),
}

/// States or events that no longer match the recorded history.
#[derive(Clone, Debug)]
pub(crate) enum StateFault {
    TamperedState,
    StalePublication(u8),
}

pub(crate) fn request_fault() -> impl proptest::strategy::Strategy<Value = RequestFault> {
    proptest::prop_oneof![
        proptest::strategy::Just(RequestFault::CrossedCampaign),
        proptest::strategy::Strategy::prop_map(proptest::num::u8::ANY, RequestFault::StaleState),
        proptest::strategy::Strategy::prop_map(1..=MAX_SKEW, RequestFault::StaleGeneration),
        proptest::strategy::Strategy::prop_map(1..=MAX_SKEW, RequestFault::AdvancedProjection),
        proptest::strategy::Just(RequestFault::StaleHead),
        proptest::strategy::Just(RequestFault::CrossedRequestKind),
    ]
}

pub(crate) fn entropy_fault() -> impl proptest::strategy::Strategy<Value = EntropyFault> {
    proptest::prop_oneof![
        proptest::strategy::Just(EntropyFault::Missing),
        proptest::strategy::Strategy::prop_map(
            crate::suite::strategies::draws(),
            EntropyFault::CrossedState
        ),
        proptest::strategy::Strategy::prop_map(
            crate::suite::strategies::draws(),
            EntropyFault::OfferedToRanked
        ),
    ]
}

pub(crate) fn state_fault() -> impl proptest::strategy::Strategy<Value = StateFault> {
    proptest::prop_oneof![
        proptest::strategy::Just(StateFault::TamperedState),
        proptest::strategy::Strategy::prop_map(
            proptest::num::u8::ANY,
            StateFault::StalePublication
        ),
    ]
}

pub(crate) fn ranked(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionRequest, String> {
    Ok(crate::suite::requests::request(
        state,
        crate::suite::requests::profile(campaign_core::SelectionMode::Ranked)?,
        None,
    ))
}

/// One checkpoint accepted before the current state, when the history has one.
fn earlier(
    history: &crate::suite::interpreter::History,
    pick: u8,
) -> Option<&campaign_core::FrontierState> {
    let (_current, earlier) = history.checkpoints.split_last()?;
    let index = crate::suite::fixture::position(pick, earlier.len()).ok()?;
    earlier.get(index)
}

pub(crate) fn stale_state_id() -> campaign_core::Id {
    crate::suite::fixture::id(campaign_core::IdKind::State, b"stale-state", 0)
}

fn denial(
    expected: campaign_core::CampaignError,
    outcome: campaign_core::CampaignResult<campaign_core::DecisionPlan>,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    Ok(Some(crate::suite::rejection::Outcome {
        expected,
        actual: crate::suite::fixture::denied(outcome)?,
    }))
}

/// Plans from the current state with one corrupted request or projection fact.
pub(crate) fn attempt_request(
    history: &crate::suite::interpreter::History,
    fault: &RequestFault,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let state = history.state()?;
    let mut request = ranked(state)?;
    let mut projection = state.admission.projection.clone();
    let expected = match fault {
        RequestFault::CrossedCampaign => {
            request.campaign_id =
                crate::suite::fixture::id(campaign_core::IdKind::Campaign, b"other", 0);
            campaign_core::CampaignError::StaleState
        }
        RequestFault::StaleState(pick) => {
            request.state_id =
                earlier(history, *pick).map_or_else(stale_state_id, |item| item.state_id);
            campaign_core::CampaignError::StaleState
        }
        RequestFault::StaleGeneration(skew) => {
            request.expected_generation = request.expected_generation.saturating_add(*skew);
            campaign_core::CampaignError::StaleProjection
        }
        RequestFault::AdvancedProjection(skew) => {
            projection.generation = projection.generation.saturating_add(*skew);
            projection = crate::suite::fixture::observed(projection.derive())?;
            campaign_core::CampaignError::StaleProjection
        }
        RequestFault::StaleHead => {
            request.expected_head_id =
                crate::suite::fixture::id(campaign_core::IdKind::Head, b"moved", 0);
            campaign_core::CampaignError::StaleProjection
        }
        RequestFault::CrossedRequestKind => {
            request.request_id =
                crate::suite::fixture::id(campaign_core::IdKind::Candidate, b"request", 0);
            campaign_core::CampaignError::CrossedIdentity
        }
    };
    assert_eq!(request.profile, ranked(state)?.profile);
    assert_eq!(
        projection.projection_schema_id,
        state.admission.projection.projection_schema_id
    );
    denial(
        expected,
        campaign_core::plan_next(state, request, &projection),
    )
}

/// Plans from the current state with a missing, crossed, or unexpected entropy ticket.
pub(crate) fn attempt_entropy(
    history: &crate::suite::interpreter::History,
    exploration: &crate::suite::strategies::Exploration,
    fault: &EntropyFault,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let state = history.state()?;
    let exploratory = crate::suite::requests::exploratory(
        state,
        exploration.threshold,
        exploration.order.clone(),
    )?;
    let request = match fault {
        EntropyFault::Missing => crate::suite::requests::request(state, exploratory, None),
        EntropyFault::CrossedState(draws) => {
            let mut other = state.clone();
            other.state_id = stale_state_id();
            let ticket = crate::suite::requests::ticket(&other, &exploratory, *draws, 0)?;
            assert_ne!(ticket.state_id, state.state_id);
            crate::suite::requests::request(state, exploratory, Some(ticket))
        }
        EntropyFault::OfferedToRanked(draws) => {
            let mut request = ranked(state)?;
            request.entropy = Some(crate::suite::requests::ticket(
                state,
                &request.profile,
                *draws,
                0,
            )?);
            request
        }
    };
    assert_eq!(request.state_id, state.state_id);
    denial(
        campaign_core::CampaignError::InvalidEntropy,
        crate::suite::requests::plan(state, request),
    )
}

/// Plans from a tampered state, or publishes a selection planned from an earlier state.
pub(crate) fn attempt_state(
    history: &crate::suite::interpreter::History,
    fault: &StateFault,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let state = history.state()?;
    match fault {
        StateFault::TamperedState => {
            let mut tampered = state.clone();
            tampered.selection_count = tampered.selection_count.saturating_add(1);
            assert_eq!(tampered.state_id, state.state_id);
            denial(
                campaign_core::CampaignError::StaleState,
                crate::suite::requests::plan(&tampered, ranked(state)?),
            )
        }
        StateFault::StalePublication(pick) => stale_publication(history, *pick),
    }
}

/// Appends a selection planned from an earlier state after the history moved on.
fn stale_publication(
    history: &crate::suite::interpreter::History,
    pick: u8,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let Some(before) = earlier(history, pick) else {
        return Ok(None);
    };
    let request = ranked(before)?;
    let profile = request.profile.clone();
    let plan = crate::suite::fixture::observed(crate::suite::requests::plan(before, request))?;
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Ok(None);
    };
    let state = history.state()?;
    assert_ne!(selection.state_before_id, state.state_id);
    let event_id = selection.publication_plan.event_id;
    let expected = if state.event_ids.contains(&event_id) {
        campaign_core::CampaignError::DuplicateControlEvent
    } else {
        campaign_core::CampaignError::StaleState
    };
    let mut events = history.events.clone();
    events.push(campaign_core::CampaignControlEvent::SelectionPublished(
        crate::suite::fixture::observed(campaign_core::SelectionEvent::new(
            *selection, profile, None,
        ))?,
    ));
    assert_eq!(events.len(), history.events.len().saturating_add(1));
    Ok(Some(crate::suite::rejection::Outcome {
        expected,
        actual: crate::suite::fixture::denied(campaign_core::reconstruct(&events))?,
    }))
}
