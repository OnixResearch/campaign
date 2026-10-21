//! `campaign.wait.guidance` / `campaign.wait.guidance_exhausted` (D69): an unguided candidate is
//! awaited only while the control events admit its guidance, a selection, that selection's
//! settlement, and the stop (`count + outstanding + 4 <= max_control_events`); otherwise the
//! planner waits for outstanding results or stops at the control-event limit, and `admit_stop`
//! agrees.

/// An admitted campaign under `max_control_events` with one unguided candidate.
fn one_unguided(
    max_control_events: u64,
) -> Result<
    (
        crate::suite::s3_fixture::History,
        campaign_core::FrontierState,
        campaign_core::Id,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_control_events = max_control_events;
    })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"room-unguided",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(state.control_event_count, 2);
    assert!(state.outstanding.is_empty());
    Ok((events, state, candidate_id))
}

fn stop_denial(
    state: &campaign_core::FrontierState,
    reason: campaign_core::StopReason,
) -> Result<campaign_core::CampaignError, String> {
    crate::suite::s3_fixture::denial(campaign_core::admit_stop(state, reason))
}

/// At `count + outstanding + 3 = max_control_events` guidance would consume the last selection
/// slot: the planner stops at the control-event limit instead of awaiting guidance, `admit_stop`
/// accepts exactly that stop, and guidance admitted anyway still leaves the candidate unselectable.
#[test]
// r[verify campaign.wait.guidance_room]
// r[verify campaign.stop.control_event_limit]
fn guidance_without_room_to_select_is_not_awaited() -> Result<(), String> {
    let (events, state, candidate_id) = one_unguided(5)?;
    let stop = crate::suite::s3_fixture::planned_stop(&state)?;
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::ControlEventLimitReached
    );
    assert_eq!(
        stop_denial(&state, campaign_core::StopReason::FrontierExhausted)?,
        campaign_core::CampaignError::InvalidStop
    );
    let accepted =
        campaign_core::admit_stop(&state, campaign_core::StopReason::ControlEventLimitReached)
            .map_err(crate::suite::s3_fixture::err)?;
    let (_stopped_events, stopped) = crate::suite::s3_fixture::append(events.clone(), accepted)?;
    assert_eq!(
        stopped.stopped.map(|stop| stop.reason),
        Some(campaign_core::StopReason::ControlEventLimitReached)
    );
    let (_events, guided, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&guided)?.reason,
        campaign_core::StopReason::ControlEventLimitReached
    );
    Ok(())
}

/// One control event more (`count + outstanding + 4 = max_control_events`) and the candidate is
/// awaited, no bound stop is admissible, and once guided it is selected.
#[test]
// r[verify campaign.wait.guidance]
fn guidance_with_room_to_select_is_awaited_and_then_selected() -> Result<(), String> {
    let (events, state, candidate_id) = one_unguided(6)?;
    assert_eq!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    for reason in [
        campaign_core::StopReason::ControlEventLimitReached,
        campaign_core::StopReason::FrontierExhausted,
    ] {
        assert_eq!(
            stop_denial(&state, reason)?,
            campaign_core::CampaignError::InvalidStop
        );
    }
    let (_events, guided, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        crate::suite::selection::plan(&guided)?.candidate_id,
        candidate_id
    );
    Ok(())
}

/// With a selection outstanding and no eligible candidate, an unguided candidate without room to
/// be guided and selected is not awaited: the planner waits for the outstanding result and no
/// bound stop is admissible yet.
#[test]
// r[verify campaign.wait.guidance_room]
// r[verify campaign.wait.results]
fn an_outstanding_result_is_awaited_instead_of_unselectable_guidance() -> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_control_events = 9;
    })?;
    // More than half the budget: once selected, its own cost no longer fits the remainder.
    let (events, state, most_budget) = crate::suite::candidate::admit(
        events,
        &state,
        b"room-most-budget",
        (crate::suite::constants::BUDGET_TOTAL / 2).saturating_add(1),
    )?;
    let (events, state, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        most_budget,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
    assert_eq!(selection.candidate_id, most_budget);
    let (_events, state, _fitting) = crate::suite::candidate::admit(
        events,
        &state,
        b"room-fitting",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(state.control_event_count, 5);
    assert_eq!(state.outstanding.len(), 1);
    assert_eq!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    );
    for reason in [
        campaign_core::StopReason::ControlEventLimitReached,
        campaign_core::StopReason::FrontierExhausted,
    ] {
        assert_eq!(
            stop_denial(&state, reason)?,
            campaign_core::CampaignError::InvalidStop
        );
    }
    Ok(())
}

/// At the same control-event boundary (`count + outstanding + 3 = max_control_events`) after a
/// guided candidate was retired, under `max_guidance_records`.
fn retired_and_unguided(max_guidance_records: u64) -> Result<campaign_core::FrontierState, String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_control_events = 8;
        bounds.max_guidance_records = max_guidance_records;
    })?;
    let (events, state, retired) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"room-retired",
        crate::suite::constants::RANK_HIGH,
    )?;
    let retire = campaign_core::retire_candidate(
        &state,
        crate::suite::s3_fixture::retirement(
            &state,
            retired,
            campaign_core::RetirementOrigin::Consumer,
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, retire)?;
    let (_events, state, _unguided) = crate::suite::candidate::admit(
        events,
        &state,
        b"room-unguided",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(state.control_event_count, 5);
    assert_eq!(state.guidance_record_count, 1);
    Ok(state)
}

/// The guidance-record bound is checked first: with it used up the unguided candidate can never
/// be guided, so the frontier stop (not the control-event limit) is planned and admissible; with
/// one more record allowed, the same state stops at the control-event limit.
#[test]
// r[verify campaign.wait.guidance_exhausted]
// r[verify campaign.wait.guidance_room]
// r[verify campaign.stop.precedence]
fn a_used_up_guidance_bound_precedes_the_control_event_room() -> Result<(), String> {
    let exhausted = retired_and_unguided(1)?;
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&exhausted)?.reason,
        campaign_core::StopReason::FrontierExhausted
    );
    assert_eq!(
        stop_denial(
            &exhausted,
            campaign_core::StopReason::ControlEventLimitReached
        )?,
        campaign_core::CampaignError::InvalidStop
    );
    let open = retired_and_unguided(2)?;
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&open)?.reason,
        campaign_core::StopReason::ControlEventLimitReached
    );
    assert_eq!(
        stop_denial(&open, campaign_core::StopReason::FrontierExhausted)?,
        campaign_core::CampaignError::InvalidStop
    );
    Ok(())
}
