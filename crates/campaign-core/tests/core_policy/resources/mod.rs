#[test]
// r[verify campaign.wait]
// r[verify campaign.wait.guidance]
// r[verify campaign.stop.precedence]
fn waits_precede_frontier_and_budget_stops() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (_events, state, _candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"needs-guidance",
        crate::suite::constants::COST_ONE,
    )?;
    let wait = (campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state, crate::suite::selection::ranked_profile()?, None),
        &state.admission.projection,
    ))
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        wait.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    let event = (campaign_core::admit_campaign(crate::suite::admission::with_budget(
        crate::suite::admission::Limits {
            budget: crate::suite::constants::BUDGET_TOTAL,
            max_outstanding: crate::suite::constants::BOUND_OUTSTANDING_ONE,
        },
    )?))
    .map_err(|error| format!("{error:?}"))?;
    let events = vec![event];
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"small",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (_events, state) = crate::suite::selection::publish(events, selection)?;
    let wait = (campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state, crate::suite::selection::ranked_profile()?, None),
        &state.admission.projection,
    ))
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        wait.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity)
    );
    Ok(())
}
#[test]
// r[verify campaign.budgets.no_refund]
fn budgets_are_reserved_without_refund() -> Result<(), String> {
    let event = (campaign_core::admit_campaign(crate::suite::admission::with_budget(
        crate::suite::admission::Limits {
            budget: crate::suite::constants::BUDGET_SMALL,
            max_outstanding: crate::suite::constants::BOUND_MANY,
        },
    )?))
    .map_err(|error| format!("{error:?}"))?;
    let events = vec![event];
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"budget",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (mut events, state) = crate::suite::selection::publish(events, selection.clone())?;
    let mut closure = campaign_core::UnobservedClosure {
        closure_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"uncertain"),
        uncertainty_acknowledged: true,
    };
    closure.closure_id =
        (campaign_core::derive_closure_id(&closure)).map_err(|error| format!("{error:?}"))?;
    events.push(
        (campaign_core::close_unobserved(&state, closure)).map_err(|error| format!("{error:?}"))?,
    );
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        state.reserved_resources[0].amount,
        crate::suite::constants::BUDGET_SMALL
    );
    let decision = campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state, crate::suite::selection::ranked_profile()?, None),
        &state.admission.projection,
    )
    .map_err(|error| format!("{error:?}"))?
    .decision;
    let campaign_core::NextDecision::Stop(stop) = decision else {
        return Err(format!("expected budget stop, got {decision:?}"));
    };
    assert!(matches!(
        stop.reason,
        campaign_core::StopReason::BudgetExhausted { .. }
    ));
    Ok(())
}
