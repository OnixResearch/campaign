//! Control-event headroom and budget remainder arithmetic.

/// Headroom keeps a stop and every outstanding settlement recordable: at the bound the planner
/// waits for results, new work is refused before an unreplayable event, and after a consumer stop
/// the outstanding selection still settles.
#[test]
// r[verify campaign.budgets.control_event_reserve]
// r[verify campaign.stop.control_event_limit]
fn outstanding_work_and_a_stop_stay_recordable_at_the_control_event_bound() -> Result<(), String> {
    let (events, state) =
        crate::suite::s3_fixture::admitted_with(|bounds| bounds.max_control_events = 6)?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"audit-strand")?;
    assert_eq!(state.control_event_count, 4);
    assert_eq!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(crate::suite::ingress::support::admit_batch(
            &state,
            vec![crate::suite::world::observed(
                crate::suite::candidate::record(
                    &state,
                    b"audit-late",
                    crate::suite::constants::COST_ONE
                )
                .derive()
            )?],
            crate::suite::ingress::support::COALESCE,
        ))?,
        campaign_core::CampaignError::ControlEventLimitReached
    );
    let stop = campaign_core::admit_stop(
        &state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(
                campaign_core::IdKind::Reason,
                b"audit-bound-stop",
            ),
        },
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, stopped) = crate::suite::s3_fixture::append(events, stop)?;
    assert_eq!(stopped.control_event_count, 5);
    let result = campaign_core::admit_result(
        &stopped,
        crate::suite::s3_fixture::observation(&stopped, &selection, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let settled = campaign_core::apply_control_event(stopped.clone(), &result)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(settled.control_event_count, 6);
    assert!(settled.outstanding.is_empty());
    let closure = campaign_core::close_unobserved(
        &stopped,
        crate::suite::s3_fixture::closure(&stopped, &selection, b"audit-bound-close", true)?,
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, closed) = crate::suite::s3_fixture::append(events, closure)?;
    assert!(closed.outstanding.is_empty());
    Ok(())
}

/// Without headroom for a selection, its settlement, and a stop, the planner requests the
/// control-event-limit stop instead of selecting; that stop is admissible and a forged selection
/// is refused by replay.
#[test]
// r[verify campaign.stop.control_event_limit]
// r[verify campaign.stop.precedence]
fn the_planner_stops_at_the_control_event_bound_instead_of_selecting() -> Result<(), String> {
    let (events, state) =
        crate::suite::s3_fixture::admitted_with(|bounds| bounds.max_control_events = 5)?;
    let (events, state, candidate) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"audit-bound",
        crate::suite::constants::RANK_HIGH,
    )?;
    assert_eq!(state.control_event_count, 3);
    let stop = crate::suite::s3_fixture::planned_stop(&state)?;
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::ControlEventLimitReached
    );
    let ranked = crate::suite::selection::ranked_profile()?;
    let forged = super::selection_of(&state, candidate, &ranked)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&events, crate::suite::selection::event(forged)?)?,
        campaign_core::CampaignError::ControlEventLimitReached
    );
    let accepted =
        campaign_core::admit_stop(&state, campaign_core::StopReason::ControlEventLimitReached)
            .map_err(crate::suite::s3_fixture::err)?;
    let (_events, stopped) = crate::suite::s3_fixture::append(events, accepted)?;
    assert_eq!(
        stopped.stopped.map(|stop| stop.reason),
        Some(campaign_core::StopReason::ControlEventLimitReached)
    );
    Ok(())
}

/// Bounds that leave no room for the admission event and a stop are invalid.
#[test]
fn a_control_event_bound_without_a_stop_reserve_is_invalid() -> Result<(), String> {
    let mut input = crate::suite::admission::default_input()?;
    input.bounds.max_control_events = 1;
    assert_eq!(
        crate::suite::s3_fixture::denial(input.bounds.clone().derive())?,
        campaign_core::CampaignError::InvalidConfig
    );
    input.bounds.max_control_events = campaign_core::MIN_CONTROL_EVENTS;
    crate::suite::world::observed(input.bounds.derive())?;
    Ok(())
}

/// With a full u64 budget consumed, the fit check compares against the remaining budget, so the
/// planner and `admit_stop` record the budget-exhausted stop.
#[test]
// r[verify campaign.budgets.exhausted]
// r[verify campaign.budgets.fit]
fn a_consumed_u64_budget_stops_with_budget_exhausted() -> Result<(), String> {
    let (events, state) = crate::suite::admission::admitted(crate::suite::admission::with_budget(
        crate::suite::admission::Limits {
            budget: u64::MAX,
            max_outstanding: crate::suite::constants::BOUND_MANY,
        },
    )?)?;
    let (events, state, candidate) =
        crate::suite::candidate::admit(events, &state, b"audit-max-cost", u64::MAX)?;
    let (events, state, _g) = crate::suite::candidate::guide(
        events,
        &state,
        candidate,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
    let result = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &selection, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, settled) = crate::suite::s3_fixture::append(events, result)?;
    assert!(settled.outstanding.is_empty());
    let dimension = settled.admission.resource_budgets[0].dimension_id;
    let reason = campaign_core::StopReason::BudgetExhausted {
        dimensions: vec![dimension],
    };
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&settled)?.reason,
        reason
    );
    let stop = campaign_core::admit_stop(&settled, reason.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let (_events, stopped) = crate::suite::s3_fixture::append(events, stop)?;
    assert_eq!(stopped.stopped.map(|stop| stop.reason), Some(reason));
    Ok(())
}
