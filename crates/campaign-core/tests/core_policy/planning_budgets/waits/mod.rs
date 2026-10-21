//! Budget and guidance waits (D61): a retained candidate awaits guidance only while the guidance
//! bound admits a record AND its declared costs fit the remaining budgets; the budget stop names
//! every dimension some retained candidate, guided or not, can no longer fit.

/// Time left (16 - 3) after the guided `heavy` candidate reserved its time cost.
const TIME_REMAINING: u64 = 13;
const TIME_OVER_REMAINING: u64 = 14;

/// A guided `heavy` candidate (time 3, memory 3 of 3) and an unguided candidate declaring
/// `unguided_time`; `heavy` is selected and published, so memory is fully reserved.
fn heavy_published(
    unguided_time: u64,
) -> Result<
    (
        crate::suite::planning_support::History,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let heavy = vec![
        crate::suite::planning_budgets::amount(
            crate::suite::planning_budgets::time(),
            crate::suite::planning_budgets::TIME_COST,
        ),
        crate::suite::planning_budgets::amount(
            crate::suite::planning_budgets::memory(),
            crate::suite::planning_budgets::MEMORY_BUDGET,
        ),
    ];
    let ((events, state), heavy_id) = crate::suite::planning_budgets::guided_costed(
        crate::suite::planning_budgets::two_dimensions()?,
        b"heavy",
        heavy,
        crate::suite::planning_budgets::RANK_TOP,
    )?;
    let (events, state, _unguided) = crate::suite::planning_support::admit_costed(
        events,
        &state,
        b"unguided",
        vec![crate::suite::planning_budgets::amount(
            crate::suite::planning_budgets::time(),
            unguided_time,
        )],
    )?;
    let (events, state, selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(selection.candidate_id, heavy_id);
    assert_eq!(
        crate::suite::planning_budgets::TIME_BUDGET
            .checked_sub(crate::suite::planning_budgets::TIME_COST),
        Some(TIME_REMAINING)
    );
    assert_eq!(TIME_REMAINING.checked_add(1), Some(TIME_OVER_REMAINING));
    assert!(state.guidance_record_count < state.admission.bounds.max_guidance_records);
    Ok(((events, state), selection))
}

fn settled(
    ((events, state), selection): (
        crate::suite::planning_support::History,
        campaign_core::SelectionDecision,
    ),
) -> Result<campaign_core::FrontierState, String> {
    let (_events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &selection,
        (b"heavy-child", b"heavy-payload"),
        Vec::new(),
    )?;
    assert!(state.outstanding.is_empty());
    Ok(state)
}

fn stop(
    state: &campaign_core::FrontierState,
    reason: campaign_core::StopReason,
) -> campaign_core::CampaignResult<campaign_core::CampaignControlEvent> {
    campaign_core::admit_stop(state, reason)
}

/// An unguided candidate that cannot fit the remaining time budget is not awaited: with the guided
/// candidate blocked in memory, the planner stops with both blocking dimensions and `admit_stop`
/// accepts exactly that reason.
#[test]
// r[verify campaign.wait.guidance_exhausted]
// r[verify campaign.budgets.exhausted]
// r[verify campaign.stop.budget]
fn an_all_over_budget_frontier_stops_with_every_blocking_dimension() -> Result<(), String> {
    let published = heavy_published(TIME_OVER_REMAINING)?;
    assert_eq!(
        crate::suite::planning_support::ranked(&published.0.1)?.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    );
    let state = settled(published)?;
    let mut both = vec![
        crate::suite::planning_budgets::time(),
        crate::suite::planning_budgets::memory(),
    ];
    both.sort();
    let expected = campaign_core::StopReason::BudgetExhausted { dimensions: both };
    let planned =
        crate::suite::planning_support::stopped(&crate::suite::planning_support::ranked(&state)?)?;
    assert_eq!(planned.reason, expected);
    for refused in [
        campaign_core::StopReason::FrontierExhausted,
        campaign_core::StopReason::BudgetExhausted {
            dimensions: vec![crate::suite::planning_budgets::memory()],
        },
    ] {
        assert_eq!(
            crate::suite::planning_support::denied(stop(&state, refused))?,
            campaign_core::CampaignError::InvalidStop
        );
    }
    let event = crate::suite::planning_support::observed(stop(&state, expected.clone()))?;
    let stopped = crate::suite::planning_support::observed(campaign_core::apply_control_event(
        state, &event,
    ))?;
    assert_eq!(stopped.stopped.map(|stop| stop.reason), Some(expected));
    Ok(())
}

/// An unguided candidate whose costs still fit the remaining budgets is awaited, and no budget
/// stop is admissible for it.
#[test]
// r[verify campaign.wait.guidance]
fn an_unguided_candidate_that_still_fits_is_awaited() -> Result<(), String> {
    let state = settled(heavy_published(TIME_REMAINING)?)?;
    assert_eq!(
        crate::suite::planning_support::ranked(&state)?.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    assert_eq!(
        crate::suite::planning_support::denied(stop(
            &state,
            campaign_core::StopReason::BudgetExhausted {
                dimensions: vec![crate::suite::planning_budgets::memory()],
            },
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    Ok(())
}
