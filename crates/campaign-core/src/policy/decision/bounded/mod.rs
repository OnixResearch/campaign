//! Decisions at a bound: selection, control-event, and finding limits, and an empty frontier.

/// Without headroom for one more selection, its settlement, and a stop, the planner waits for
/// outstanding results or, with nothing outstanding, requests the control-event-limit stop.
// r[impl campaign.stop.control_event_limit]
pub(super) fn plan_at_control_event_limit(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    if !state.outstanding.is_empty() {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingResults),
            crate::model::PlanningWork::none(),
        ));
    }
    let stop = crate::policy::plans::build_stop(
        state,
        crate::model::StopReason::ControlEventLimitReached,
    )?;
    debug_assert!(state.outstanding.is_empty());
    Ok(crate::policy::plans::decision_plan(
        state,
        profile,
        crate::model::NextDecision::Stop(alloc::boxed::Box::new(stop)),
        crate::model::PlanningWork::none(),
    ))
}

// r[impl campaign.stop.selection_limit]
pub(super) fn plan_at_selection_limit(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    debug_assert!(state.selection_count >= state.admission.bounds.max_selections);
    if !state.outstanding.is_empty() {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingResults),
            crate::model::PlanningWork::none(),
        ));
    }
    let stop =
        crate::policy::plans::build_stop(state, crate::model::StopReason::SelectionLimitReached)?;
    Ok(crate::policy::plans::decision_plan(
        state,
        profile,
        crate::model::NextDecision::Stop(alloc::boxed::Box::new(stop)),
        crate::model::PlanningWork::none(),
    ))
}

/// Applies the declared `StoppingRule`: once accepted findings reach the finding limit the
/// planner waits for outstanding results, then requests the rule's policy stop.
// r[impl campaign.findings.non_terminal]
// r[impl campaign.findings.valid]
pub(super) fn plan_at_finding_limit(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<Option<crate::model::DecisionPlan>> {
    let Some(finding_limit) = profile.stopping.finding_limit else {
        return Ok(None);
    };
    if crate::identity::usize_to_u64(state.findings.len())? < finding_limit {
        return Ok(None);
    }
    if !state.outstanding.is_empty() {
        return Ok(Some(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingResults),
            crate::model::PlanningWork::none(),
        )));
    }
    let reason_id = profile
        .stopping
        .policy_reason_id()?
        .ok_or(crate::error::CampaignError::InvalidConfig)?;
    let stop = crate::policy::plans::build_stop(
        state,
        crate::model::StopReason::PolicyRequested { reason_id },
    )?;
    debug_assert!(state.outstanding.is_empty());
    Ok(Some(crate::policy::plans::decision_plan(
        state,
        profile,
        crate::model::NextDecision::Stop(alloc::boxed::Box::new(stop)),
        crate::model::PlanningWork::none(),
    )))
}

/// Without an eligible candidate the planner awaits guidance a candidate can still use, then
/// outstanding results; with nothing outstanding it stops at the control-event limit when only
/// that limit keeps an unguided candidate from selection, else at the frontier or budget.
// r[impl campaign.stop]
// r[impl campaign.wait]
// r[impl campaign.wait.results]
// r[impl campaign.stop.frontier]
// r[impl campaign.stop.budget]
// r[impl campaign.wait.guidance_room]
// r[impl campaign.stop.control_event_limit]
pub(super) fn plan_without_eligible(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
    work: crate::model::PlanningWork,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    let guidance_wait = crate::policy::capacity::guidance_wait(state)?;
    if guidance_wait == crate::policy::capacity::GuidanceWait::Awaiting {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingGuidance),
            work,
        ));
    }
    if !state.outstanding.is_empty() {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingResults),
            work,
        ));
    }
    debug_assert!(state.outstanding.is_empty());
    let reason = if guidance_wait == crate::policy::capacity::GuidanceWait::ControlEventLimited {
        crate::model::StopReason::ControlEventLimitReached
    } else {
        let dimensions = crate::policy::capacity::exhausted_dimensions(state)?;
        if dimensions.is_empty() {
            crate::model::StopReason::FrontierExhausted
        } else {
            crate::model::StopReason::BudgetExhausted { dimensions }
        }
    };
    let stop = crate::policy::plans::build_stop(state, reason)?;
    Ok(crate::policy::plans::decision_plan(
        state,
        profile,
        crate::model::NextDecision::Stop(alloc::boxed::Box::new(stop)),
        work,
    ))
}
