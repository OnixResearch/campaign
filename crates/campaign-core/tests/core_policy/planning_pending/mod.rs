//! Explicit pending-input waits (`campaign.wait`); waits are never stops.

const RANK_TOP: i64 = 20;
const RANK_BOTTOM: i64 = 10;

fn wait_reason(plan: &campaign_core::DecisionPlan) -> Result<campaign_core::WaitReason, String> {
    let campaign_core::NextDecision::Wait(reason) = &plan.decision else {
        return Err(format!("expected a wait, got {:?}", plan.decision));
    };
    Ok(reason.clone())
}

#[test]
// r[verify campaign.wait.guidance]
fn an_unguided_candidate_waits_for_guidance_instead_of_exhausting() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, retired) =
        crate::suite::planning_support::guided(events, &state, b"retired", RANK_TOP)?;
    let (_events, state, _pending) = crate::suite::candidate::admit(
        events,
        &state,
        b"pending",
        crate::suite::constants::COST_ONE,
    )?;
    let state = crate::suite::planning_support::rebuilt(state, |forged| {
        for candidate in &mut forged.candidates {
            if candidate.candidate.candidate_id == retired {
                candidate.status = campaign_core::CandidateStatus::Retired {
                    reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"gone"),
                };
            }
        }
    })?;
    let plan = crate::suite::planning_support::ranked(&state)?;
    assert_eq!(
        wait_reason(&plan)?,
        campaign_core::WaitReason::AwaitingGuidance
    );
    assert_eq!(plan.work.eligible_candidates, 0);
    Ok(())
}

#[test]
// r[verify campaign.wait.results]
fn outstanding_work_waits_for_results_instead_of_stopping() -> Result<(), String> {
    let (events, state) = crate::suite::planning_support::admitted_with(|input| {
        input.resource_budgets[0].amount = crate::suite::constants::BUDGET_SMALL;
    })?;
    let (events, state, _only) =
        crate::suite::planning_support::guided(events, &state, b"only", RANK_TOP)?;
    let (_events, state, _selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let plan = crate::suite::planning_support::ranked(&state)?;
    assert_eq!(
        wait_reason(&plan)?,
        campaign_core::WaitReason::AwaitingResults
    );
    assert_eq!(plan.work.eligible_candidates, 0);
    Ok(())
}

#[test]
// r[verify campaign.wait.capacity]
fn a_full_outstanding_bound_waits_without_reserving_budget() -> Result<(), String> {
    let (events, state) = crate::suite::planning_support::admitted_with(|input| {
        input.bounds.max_outstanding = crate::suite::constants::BOUND_OUTSTANDING_ONE;
    })?;
    let (events, state, _top) =
        crate::suite::planning_support::guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, _bottom) =
        crate::suite::planning_support::guided(events, &state, b"bottom", RANK_BOTTOM)?;
    let (events, state, _selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let plan = crate::suite::planning_support::ranked(&state)?;
    assert_eq!(
        wait_reason(&plan)?,
        campaign_core::WaitReason::AwaitingCapacity
    );
    assert!(plan.work.eligible_candidates > 0);
    let after = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(after, state);
    assert_eq!(
        after.selection_count,
        crate::suite::planning_support::FIRST_ORDINAL
    );
    assert_eq!(after.reserved_resources, state.reserved_resources);
    Ok(())
}
