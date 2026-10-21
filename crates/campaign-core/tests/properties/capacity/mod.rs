//! Outstanding-capacity exclusion: outstanding selections never exceed `max_outstanding`; at the
//! bound the planner waits for capacity and emits no selection, so nothing is reserved.

fn outstanding_bound_is_exclusive(
    history: &crate::suite::interpreter::History,
    at_bound: &core::cell::Cell<u64>,
) -> Result<(), String> {
    for state in &history.checkpoints {
        let outstanding =
            u64::try_from(state.outstanding.len()).map_err(|error| format!("{error}"))?;
        assert!(outstanding <= state.admission.bounds.max_outstanding);
    }
    for planning in &history.plannings {
        let state = &planning.state;
        let decided = crate::suite::oracle::kind(&planning.plan.decision);
        assert_eq!(decided, crate::suite::oracle::expected_kind(state)?);
        let outstanding =
            u64::try_from(state.outstanding.len()).map_err(|error| format!("{error}"))?;
        if outstanding < state.admission.bounds.max_outstanding {
            continue;
        }
        assert!(
            !planning.published,
            "a plan at the outstanding bound published a selection"
        );
        assert_ne!(decided, crate::suite::oracle::Kind::Select);
        if decided == crate::suite::oracle::Kind::Wait(campaign_core::WaitReason::AwaitingCapacity)
        {
            assert!(planning.plan.work.eligible_candidates > 0);
            assert!(state.selection_count < state.admission.bounds.max_selections);
            at_bound.set(at_bound.get().saturating_add(1));
        }
    }
    Ok(())
}

#[test]
// r[verify campaign.wait.capacity]
// r[verify campaign.stop.precedence]
fn outstanding_selections_never_exceed_their_bound() -> Result<(), String> {
    let at_bound = core::cell::Cell::new(0_u64);
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &crate::suite::strategies::script(),
        |script| {
            outstanding_bound_is_exclusive(&crate::suite::interpreter::run(&script)?, &at_bound)
        },
    )?;
    assert!(
        at_bound.get() > 0,
        "no generated history planned at the outstanding bound"
    );
    Ok(())
}
