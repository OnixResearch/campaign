//! Stop admission preconditions for frontier, budget, selection-limit, and consumer stops.

mod finality;

pub(crate) fn stop_event(
    state: &campaign_core::FrontierState,
    stop: campaign_core::StopDecision,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::StopAccepted,
        state.admission.campaign_id,
        state.state_id,
        &[stop.stop_id],
    )
    .map_err(crate::suite::s3_fixture::err)?;
    Ok(campaign_core::CampaignControlEvent::StopAccepted(
        campaign_core::StopEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            stop,
        },
    ))
}

pub(crate) fn forged_stop(
    state: &campaign_core::FrontierState,
    campaign_id: campaign_core::Id,
    reason: campaign_core::StopReason,
    acknowledged: bool,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut stop = campaign_core::StopDecision {
        stop_id: campaign_core::Id::absent(),
        campaign_id,
        state_before_id: state.state_id,
        reason,
        incomplete_search_acknowledged: acknowledged,
    };
    stop.stop_id = campaign_core::derive_stop_id(&stop).map_err(crate::suite::s3_fixture::err)?;
    stop_event(state, stop)
}

pub(crate) fn forged_reason(
    state: &campaign_core::FrontierState,
    reason: campaign_core::StopReason,
) -> Result<campaign_core::CampaignControlEvent, String> {
    forged_stop(state, state.admission.campaign_id, reason, true)
}

pub(crate) fn consumer_reason() -> campaign_core::StopReason {
    campaign_core::StopReason::ConsumerRequested {
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"consumer-stop"),
    }
}

pub(crate) fn accepted(
    state: &campaign_core::FrontierState,
    reason: campaign_core::StopReason,
) -> Result<campaign_core::StopEvent, String> {
    match campaign_core::admit_stop(state, reason).map_err(crate::suite::s3_fixture::err)? {
        campaign_core::CampaignControlEvent::StopAccepted(event) => Ok(event),
        other => Err(format!("expected a stop event, got {other:?}")),
    }
}

pub(crate) fn budget_dimension(state: &campaign_core::FrontierState) -> campaign_core::StopReason {
    campaign_core::StopReason::BudgetExhausted {
        dimensions: vec![state.admission.resource_budgets[0].dimension_id],
    }
}

#[test]
// r[verify campaign.stop]
// r[verify campaign.stop.frontier]
fn a_frontier_stop_is_admissible_only_when_the_planner_would_stop() -> Result<(), String> {
    let (events, empty) = crate::suite::admission::initial_state()?;
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&empty)?.reason,
        campaign_core::StopReason::FrontierExhausted
    );
    for wrong in [
        budget_dimension(&empty),
        campaign_core::StopReason::SelectionLimitReached,
    ] {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::admit_stop(&empty, wrong))?,
            campaign_core::CampaignError::InvalidStop
        );
    }
    let stop = accepted(&empty, campaign_core::StopReason::FrontierExhausted)?;
    let (_stopped_events, stopped) = crate::suite::s3_fixture::append(
        events.clone(),
        campaign_core::CampaignControlEvent::StopAccepted(stop),
    )?;
    assert!(stopped.stopped.is_some());
    let (events, unguided, candidate_id) = crate::suite::candidate::admit(
        events,
        &empty,
        b"unguided",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &unguided,
            campaign_core::StopReason::FrontierExhausted
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    let (events, guided, _guidance) = crate::suite::candidate::guide(
        events,
        &unguided,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &guided,
            campaign_core::StopReason::FrontierExhausted
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            forged_reason(&guided, campaign_core::StopReason::FrontierExhausted)?
        )?,
        campaign_core::CampaignError::InvalidStop
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &guided)
}

#[test]
// r[verify campaign.stop.budget]
fn a_budget_stop_names_exactly_the_blocking_dimensions() -> Result<(), String> {
    let (events, state) = crate::suite::admission::admitted(crate::suite::admission::with_budget(
        crate::suite::admission::Limits {
            budget: crate::suite::constants::BUDGET_SMALL,
            max_outstanding: crate::suite::constants::BOUND_MANY,
        },
    )?)?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"budget")?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            budget_dimension(&state)
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    let closure = campaign_core::close_unobserved(
        &state,
        crate::suite::s3_fixture::closure(&state, &selection, b"budget-closure", true)?,
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, closure)?;
    let planned = crate::suite::s3_fixture::planned_stop(&state)?;
    assert_eq!(planned.reason, budget_dimension(&state));
    let wrong_reasons = [
        campaign_core::StopReason::BudgetExhausted {
            dimensions: Vec::new(),
        },
        campaign_core::StopReason::BudgetExhausted {
            dimensions: vec![crate::suite::admission::xid(
                campaign_core::IdKind::ResourceDimension,
                b"other-dimension",
            )],
        },
        campaign_core::StopReason::FrontierExhausted,
    ];
    for wrong in wrong_reasons {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::admit_stop(&state, wrong.clone()))?,
            campaign_core::CampaignError::InvalidStop
        );
        assert_eq!(
            crate::suite::s3_fixture::replay_denial(&events, forged_reason(&state, wrong)?)?,
            campaign_core::CampaignError::InvalidStop
        );
    }
    let stop = accepted(&state, budget_dimension(&state))?;
    assert_eq!(stop.stop, planned);
    crate::suite::s3_fixture::append(
        events,
        campaign_core::CampaignControlEvent::StopAccepted(stop),
    )?;
    Ok(())
}

#[test]
fn a_selection_limit_stop_requires_the_reached_limit_and_no_outstanding_work() -> Result<(), String>
{
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_selections = crate::suite::constants::COST_ONE;
    })?;
    let (events, state, _candidate) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"limit",
        crate::suite::constants::RANK_HIGH,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::SelectionLimitReached
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    let (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::SelectionLimitReached
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            forged_reason(&state, campaign_core::StopReason::SelectionLimitReached)?
        )?,
        campaign_core::CampaignError::InvalidStop
    );
    let closure = campaign_core::close_unobserved(
        &state,
        crate::suite::s3_fixture::closure(&state, &selection, b"limit-closure", true)?,
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, closure)?;
    let stop = accepted(&state, campaign_core::StopReason::SelectionLimitReached)?;
    assert_eq!(stop.stop, crate::suite::s3_fixture::planned_stop(&state)?);
    crate::suite::s3_fixture::append(
        events,
        campaign_core::CampaignControlEvent::StopAccepted(stop),
    )?;
    Ok(())
}

#[test]
// r[verify campaign.stop.consumer]
fn a_consumer_stop_keeps_outstanding_work_explicit() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"consumer")?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::ConsumerRequested {
                reason_id: crate::suite::admission::xid(
                    campaign_core::IdKind::Finding,
                    b"not-a-reason"
                ),
            }
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    accepted(
        &state,
        campaign_core::StopReason::PolicyRequested {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"policy"),
        },
    )?;
    let stop = accepted(&state, consumer_reason())?;
    assert!(stop.stop.incomplete_search_acknowledged);
    let (_events, stopped) = crate::suite::s3_fixture::append(
        events,
        campaign_core::CampaignControlEvent::StopAccepted(stop.clone()),
    )?;
    assert_eq!(stopped.outstanding.len(), 1);
    assert_eq!(stopped.outstanding[0].selection_id, selection.selection_id);
    assert_eq!(crate::suite::s3_fixture::planned_stop(&stopped)?, stop.stop);
    Ok(())
}
