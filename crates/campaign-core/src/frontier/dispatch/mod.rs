#[cfg(test)]
mod tests;

// r[impl campaign.frontier.selected]
pub(super) fn apply_selection(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::SelectionEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    let decision = &event.decision;
    if state.stopped.is_some() {
        return Err(crate::error::CampaignError::CampaignStopped);
    }
    crate::frontier::dispatch::check_selection_identity(event)?;
    if decision.campaign_id != state.admission.campaign_id
        || decision.state_before_id != state.state_id
    {
        return Err(crate::error::CampaignError::StaleState);
    }
    event.check_policy_inputs()?;
    let planned = crate::policy::decision::replan(
        &state,
        &crate::policy::decision::PlanningInputs::of_event(event),
    )?;
    let candidate_index = crate::frontier::dispatch::selected_candidate_index(&state, decision)?;
    if state.outstanding.iter().any(|selection| {
        selection.selection_id == decision.selection_id
            || selection.operation_id == decision.operation_id
    }) {
        return Err(crate::error::CampaignError::DuplicateIdentity);
    }
    if crate::identity::usize_to_u64(state.outstanding.len())?
        >= state.admission.bounds.max_outstanding
    {
        return Err(crate::error::CampaignError::OutstandingLimitReached);
    }
    state.reserved_resources = crate::frontier::dispatch::reserve_costs(
        state.reserved_resources,
        &state.admission.resource_budgets,
        &decision.reserved_costs,
    )?;
    let ordinal = crate::frontier::dispatch::next_ordinal(state.selection_count)?;
    if decision.ordinal != ordinal {
        return Err(crate::error::CampaignError::InvalidControlEventOrder);
    }
    if ordinal > state.admission.bounds.max_selections {
        return Err(crate::error::CampaignError::BoundExceeded);
    }
    if !matches!(&planned.decision, crate::model::NextDecision::Select(chosen) if **chosen == *decision)
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    state.selection_count = ordinal;
    state.candidates[candidate_index].selection_count = state.candidates[candidate_index]
        .selection_count
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert_eq!(state.selection_count, decision.ordinal);
    debug_assert_eq!(
        decision.reserved_costs,
        state.candidates[candidate_index].candidate.declared_costs
    );
    state.outstanding.push(crate::model::OutstandingSelection {
        selection_id: decision.selection_id,
        candidate_id: decision.candidate_id,
        parent_moment_id: decision.parent_moment_id,
        operation_id: decision.operation_id,
        ordinal: decision.ordinal,
        reserved_costs: decision.reserved_costs.clone(),
    });
    Ok(state)
}
/// Requires the decision to re-derive exactly, to bind the event identity, and to carry a
/// publication fence of `ControlBranch` and `Head` identities, as `plan_next` produces from a
/// validated projection binding.
fn check_selection_identity(
    event: &crate::model::SelectionEvent,
) -> crate::error::CampaignResult<()> {
    let decision = &event.decision;
    let expected = decision.clone().derive()?;
    if &expected != decision || expected.publication_plan.event_id != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let fence = &decision.publication_plan;
    fence
        .control_branch_id
        .require_kind(crate::identity::IdKind::ControlBranch)?;
    fence
        .expected_head_id
        .require_kind(crate::identity::IdKind::Head)?;
    debug_assert_eq!(fence.selection_id, decision.selection_id);
    Ok(())
}
/// Finds the selected candidate and checks it is selectable under the decision's exact parent,
/// latest guidance, and declared costs.
// r[impl campaign.selection.invalid]
fn selected_candidate_index(
    state: &crate::frontier::FrontierState,
    decision: &crate::model::SelectionDecision,
) -> crate::error::CampaignResult<usize> {
    let candidate_index =
        crate::frontier::validation::find_candidate_index(state, decision.candidate_id)?;
    let candidate = &state.candidates[candidate_index];
    crate::frontier::dispatch::ensure_candidate_selectable(candidate)?;
    if decision.parent_moment_id != candidate.candidate.parent_moment_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let guidance = candidate
        .latest_guidance
        .as_ref()
        .ok_or(crate::error::CampaignError::MissingGuidance)?;
    if guidance.guidance_id != decision.guidance_id {
        return Err(crate::error::CampaignError::StaleGuidance);
    }
    if decision.reserved_costs != candidate.candidate.declared_costs {
        return Err(crate::error::CampaignError::InvalidCostProfile);
    }
    debug_assert_eq!(candidate.candidate.candidate_id, decision.candidate_id);
    Ok(candidate_index)
}
/// Returns the checked successor of the campaign selection count: the next selection ordinal.
const fn next_ordinal(selection_count: u64) -> crate::error::CampaignResult<u64> {
    let Some(next) = selection_count.checked_add(crate::model::UNIT_COST) else {
        return Err(crate::error::CampaignError::ArithmeticOverflow);
    };
    debug_assert!(next > selection_count);
    Ok(next)
}
#[cfg_attr(
    dylint_lib = "octet",
    allow(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "POLICY owns exact typed errors for the closed candidate status contract; a new status requires an explicit rejection mapping"
    )
)]
// r[impl campaign.retirement.valid]
pub(super) const fn ensure_candidate_selectable(
    candidate: &crate::model::CandidateState,
) -> crate::error::CampaignResult<()> {
    match candidate.status {
        crate::model::CandidateStatus::Retained => Ok(()),
        crate::model::CandidateStatus::Retired { .. } => {
            Err(crate::error::CampaignError::CandidateRetired)
        }
        crate::model::CandidateStatus::Pruned { .. } => {
            Err(crate::error::CampaignError::CandidatePruned)
        }
    }
}
// r[impl campaign.budgets]
// r[impl campaign.budgets.overflow]
pub(super) fn reserve_costs(
    mut reserved: alloc::vec::Vec<crate::model::ResourceAmount>,
    budgets: &[crate::model::ResourceAmount],
    costs: &[crate::model::ResourceAmount],
) -> crate::error::CampaignResult<alloc::vec::Vec<crate::model::ResourceAmount>> {
    for cost in costs {
        let budget =
            crate::identity::collection::sorted_position(budgets, cost.dimension_id, |budget| {
                budget.dimension_id
            })
            .map(|index| &budgets[index])
            .ok_or(crate::error::CampaignError::BudgetExhausted)?;
        let current =
            crate::identity::collection::sorted_position(&reserved, cost.dimension_id, |item| {
                item.dimension_id
            })
            .map_or(crate::model::NO_SELECTIONS, |index| reserved[index].amount);
        let remaining = budget
            .amount
            .checked_sub(current)
            .ok_or(crate::error::CampaignError::BudgetExhausted)?;
        if cost.amount > remaining {
            return Err(crate::error::CampaignError::BudgetExhausted);
        }
        debug_assert!(current <= budget.amount);
    }
    reserved.extend_from_slice(costs);
    reserved.sort_by_key(|item| item.dimension_id);
    let mut summed: alloc::vec::Vec<crate::model::ResourceAmount> =
        alloc::vec::Vec::with_capacity(reserved.len());
    for item in reserved {
        if let Some(last) = summed
            .last_mut()
            .filter(|last| last.dimension_id == item.dimension_id)
        {
            last.amount = last
                .amount
                .checked_add(item.amount)
                .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
        } else {
            summed.push(item);
        }
    }
    debug_assert!(crate::identity::collection::strictly_increasing_by_key(
        &summed,
        |item| item.dimension_id
    ));
    Ok(summed)
}
