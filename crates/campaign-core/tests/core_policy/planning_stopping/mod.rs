//! Declared `StoppingRule` finding limit on the planner (`campaign.fd.scheduling`, D20).

const RANK_TOP: i64 = 20;
const RANK_BOTTOM: i64 = 10;
const ONE_FINDING: u64 = 1;
const TWO_FINDINGS: u64 = 2;
const THIRD_ORDINAL: u64 = 3;
/// Admission, two candidate batches, guidance, selection, and result use six control events; a
/// bound of nine leaves no room to guide, select, settle, and stop the unguided candidate.
const GUIDANCE_ROOM_CONTROL_EVENTS: u64 = 9;
const SIX_CONTROL_EVENTS: u64 = 6;
/// More than half of `BUDGET_TOTAL` (8).
const HEAVY_COST: u64 = 5;

fn limited(finding_limit: Option<u64>) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::planning_support::profile_with(|profile| {
        profile.stopping = campaign_core::StoppingRule { finding_limit };
    })
}

fn plan(
    state: &campaign_core::FrontierState,
    finding_limit: Option<u64>,
) -> Result<campaign_core::DecisionPlan, String> {
    crate::suite::planning_support::observed(crate::suite::planning_support::plan_with(
        state,
        limited(finding_limit)?,
        None,
    ))
}

/// Two selections outstanding; the first settles with one finding.
fn one_finding_with_one_outstanding() -> Result<
    (
        crate::suite::planning_support::History,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, _top) =
        crate::suite::planning_support::guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, _bottom) =
        crate::suite::planning_support::guided(events, &state, b"bottom", RANK_BOTTOM)?;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let (events, state, second) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let (events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &first,
        (b"finding-child", b"finding-payload"),
        vec![crate::suite::planning_support::finding(b"finding")],
    )?;
    assert_eq!(state.findings.len(), 1);
    assert_eq!(state.outstanding.len(), 1);
    Ok(((events, state), second))
}

#[test]
// r[verify campaign.fd.scheduling]
// r[verify campaign.findings.valid]
fn the_finding_limit_waits_for_outstanding_work_then_requests_its_declared_stop()
-> Result<(), String> {
    let ((events, state), second) = one_finding_with_one_outstanding()?;
    assert_eq!(
        plan(&state, Some(ONE_FINDING))?.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    );
    let (mut events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &second,
        (b"quiet-child", b"quiet-payload"),
        Vec::new(),
    )?;
    let rule = campaign_core::StoppingRule {
        finding_limit: Some(ONE_FINDING),
    };
    let reason_id = crate::suite::planning_support::observed(rule.policy_reason_id())?
        .ok_or_else(|| String::from("a finding limit must name its stop reason"))?;
    assert_eq!(reason_id.kind, campaign_core::IdKind::Reason);
    assert_eq!(
        crate::suite::planning_support::observed(rule.policy_reason_id())?,
        Some(reason_id)
    );
    assert_ne!(
        crate::suite::planning_support::observed(
            campaign_core::StoppingRule {
                finding_limit: Some(TWO_FINDINGS),
            }
            .policy_reason_id()
        )?,
        Some(reason_id)
    );
    let stop = crate::suite::planning_support::stopped(&plan(&state, Some(ONE_FINDING))?)?;
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::PolicyRequested { reason_id }
    );
    assert_eq!(stop.state_before_id, state.state_id);
    events.push(crate::suite::planning_support::observed(
        campaign_core::admit_stop(&state, stop.reason.clone()),
    )?);
    let stopped = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(stopped.stopped.as_ref(), Some(&stop));
    assert_eq!(
        crate::suite::planning_support::stopped(&plan(&stopped, None)?)?,
        stop
    );
    Ok(())
}

#[test]
// r[verify campaign.findings.non_terminal]
// r[verify campaign.findings.valid]
fn findings_below_or_without_a_limit_never_stop_selection() -> Result<(), String> {
    let ((_events, state), _second) = one_finding_with_one_outstanding()?;
    assert_eq!(
        campaign_core::StoppingRule {
            finding_limit: None
        }
        .policy_reason_id(),
        Ok(None)
    );
    for finding_limit in [None, Some(TWO_FINDINGS)] {
        let selection = crate::suite::planning_support::selected(&plan(&state, finding_limit)?)?;
        assert_eq!(selection.ordinal, THIRD_ORDINAL);
    }
    Ok(())
}

#[test]
// r[verify campaign.selection.stopped]
// r[verify campaign.stop.terminal]
fn an_accepted_stop_is_returned_and_blocks_replayed_selection() -> Result<(), String> {
    let (mut events, state, _top, _bottom) = crate::suite::planning_support::two_guided()?;
    let before_stop =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    events.push(crate::suite::planning_support::observed(
        campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::ConsumerRequested {
                reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"halt"),
            },
        ),
    )?);
    let stopped = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    let accepted = stopped
        .stopped
        .clone()
        .ok_or_else(|| String::from("the stop must be accepted"))?;
    assert_eq!(
        crate::suite::planning_support::stopped(&crate::suite::planning_support::ranked(
            &stopped
        )?)?,
        accepted
    );
    let after_stop = crate::suite::planning_support::forged_selection(&before_stop, |decision| {
        decision.state_before_id = stopped.state_id;
    })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, after_stop),
        Err(campaign_core::CampaignError::CampaignStopped)
    );
    Ok(())
}

/// D80 (`campaign.stop.control_event_limit`): `admit_stop` validates state bounds only; a profile
/// policy stop such as a reached finding limit belongs to the planner. With one accepted finding,
/// a finding limit of one, and no control-event room left for guiding and selecting the unguided
/// candidate, the planner returns the finding limit's `PolicyRequested` stop while `admit_stop`
/// still records `ControlEventLimitReached`, which the planner then returns as the accepted stop.
#[test]
// r[verify campaign.stop.control_event_limit]
fn a_reached_finding_limit_is_the_planners_stop_and_admit_stop_checks_state_bounds_only()
-> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_control_events = GUIDANCE_ROOM_CONTROL_EVENTS;
    })?;
    let (events, state, heavy) =
        crate::suite::candidate::admit(events, &state, b"finding-limit-heavy", HEAVY_COST)?;
    let (events, state, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        heavy,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
    let mut observation = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    observation.findings = vec![crate::suite::s3_fixture::finding(b"finding-limit-finding")];
    let result =
        crate::suite::planning_support::observed(campaign_core::admit_result(&state, observation))?;
    let (events, state) = crate::suite::s3_fixture::append(events, result)?;
    let (mut events, state, _light) = crate::suite::candidate::admit(
        events,
        &state,
        b"finding-limit-light",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(state.control_event_count, SIX_CONTROL_EVENTS);
    assert!(HEAVY_COST.checked_mul(2) > Some(crate::suite::constants::BUDGET_TOTAL));
    assert!(state.outstanding.is_empty());
    assert_eq!(state.findings.len(), 1);
    let reason_id = crate::suite::planning_support::observed(
        campaign_core::StoppingRule {
            finding_limit: Some(ONE_FINDING),
        }
        .policy_reason_id(),
    )?
    .ok_or_else(|| String::from("a finding limit must name its stop reason"))?;
    let planned = crate::suite::planning_support::stopped(&plan(&state, Some(ONE_FINDING))?)?;
    assert_eq!(
        planned.reason,
        campaign_core::StopReason::PolicyRequested { reason_id }
    );
    events.push(crate::suite::planning_support::observed(
        campaign_core::admit_stop(&state, campaign_core::StopReason::ControlEventLimitReached),
    )?);
    let stopped = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    let recorded = stopped
        .stopped
        .clone()
        .ok_or_else(|| String::from("the control-event stop must be accepted"))?;
    assert_eq!(
        recorded.reason,
        campaign_core::StopReason::ControlEventLimitReached
    );
    assert_eq!(recorded.state_before_id, state.state_id);
    assert_eq!(
        crate::suite::planning_support::stopped(&plan(&stopped, Some(ONE_FINDING))?)?,
        recorded
    );
    Ok(())
}
