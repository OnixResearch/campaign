/// Whether every declared cost fits the budget left in its dimension (`cost <= budget - reserved`).
// r[impl campaign.budgets]
// r[impl campaign.budgets.fit]
pub(super) fn costs_fit(
    state: &crate::frontier::FrontierState,
    costs: &[crate::model::ResourceAmount],
) -> crate::error::CampaignResult<bool> {
    for cost in costs {
        if cost.amount > remaining_budget(state, cost.dimension_id)? {
            return Ok(false);
        }
    }
    Ok(true)
}
/// What retained candidates without guidance leave the planner once no candidate is eligible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum GuidanceWait {
    /// Some unguided candidate can still be guided and selected: the planner awaits guidance.
    Awaiting,
    /// Some unguided candidate fits its budget and the guidance-record bound, but the control
    /// events no longer admit its guidance, a selection, that selection's settlement, and the
    /// stop: only the control-event limit keeps it from selection.
    ControlEventLimited,
    /// No unguided candidate could become eligible: the guidance-record bound is used up or
    /// every unguided candidate's declared costs exceed the remaining budgets.
    None,
}
/// Whether a retained candidate without guidance can still be selected once guided: the
/// guidance-record bound must still admit a record, the candidate's declared costs must fit the
/// remaining budgets, AND the control events must admit the guidance, a selection, its
/// settlement, and the stop. Otherwise no guidance could make it selectable, so it does not await
/// guidance and the planner reaches a stop instead of waiting forever.
// r[impl campaign.wait.guidance]
// r[impl campaign.wait.guidance_exhausted]
// r[impl campaign.wait.guidance_room]
pub(super) fn guidance_wait(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<GuidanceWait> {
    if state.guidance_record_count >= state.admission.bounds.max_guidance_records {
        return Ok(GuidanceWait::None);
    }
    debug_assert!(state.guidance_record_count < state.admission.bounds.max_guidance_records);
    for candidate in &state.candidates {
        if matches!(candidate.status, crate::model::CandidateStatus::Retained)
            && candidate.latest_guidance.is_none()
            && costs_fit(state, &candidate.candidate.declared_costs)?
        {
            debug_assert!(candidate.latest_guidance.is_none());
            let has_room = crate::frontier::reserve::has_control_event_headroom(
                state,
                crate::frontier::reserve::ControlEventEffect::GUIDED_SELECTION,
            )?;
            return Ok(if has_room {
                GuidanceWait::Awaiting
            } else {
                GuidanceWait::ControlEventLimited
            });
        }
    }
    Ok(GuidanceWait::None)
}
/// The sorted dimensions in which some retained candidate, guided or not, declares a cost above
/// the remaining budget. Each such dimension is marked at its position among the sorted admitted
/// budgets and the marks are collected once, so the dimensions come out sorted and unique without
/// an insertion per cost.
// r[impl campaign.budgets.exhausted]
pub(super) fn exhausted_dimensions(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<alloc::vec::Vec<crate::identity::Id>> {
    let budgets = &state.admission.resource_budgets;
    let mut exhausted = alloc::vec![false; budgets.len()];
    for candidate in &state.candidates {
        if !matches!(candidate.status, crate::model::CandidateStatus::Retained) {
            continue;
        }
        for cost in &candidate.candidate.declared_costs {
            if cost.amount <= remaining_budget(state, cost.dimension_id)? {
                continue;
            }
            let mark = crate::identity::collection::sorted_position(
                budgets,
                cost.dimension_id,
                |budget| budget.dimension_id,
            )
            .and_then(|index| exhausted.get_mut(index))
            .ok_or(crate::error::CampaignError::BudgetExhausted)?;
            *mark = true;
        }
    }
    let dimensions: alloc::vec::Vec<crate::identity::Id> = budgets
        .iter()
        .zip(&exhausted)
        .filter(|(_, is_exhausted)| **is_exhausted)
        .map(|(budget, _)| budget.dimension_id)
        .collect();
    debug_assert!(dimensions.len() <= budgets.len());
    debug_assert!(crate::identity::unique_sorted_ids(&dimensions));
    Ok(dimensions)
}
/// Budget left in a declared dimension after its reserved amount; an undeclared dimension or a
/// reservation above its budget is an impossible state.
// r[impl campaign.budgets.fit]
fn remaining_budget(
    state: &crate::frontier::FrontierState,
    dimension_id: crate::identity::Id,
) -> crate::error::CampaignResult<u64> {
    let budgets = &state.admission.resource_budgets;
    let budget = crate::identity::collection::sorted_position(budgets, dimension_id, |budget| {
        budget.dimension_id
    })
    .map(|index| &budgets[index])
    .ok_or(crate::error::CampaignError::BudgetExhausted)?;
    let reserved = crate::identity::collection::sorted_position(
        &state.reserved_resources,
        dimension_id,
        |reserved| reserved.dimension_id,
    )
    .map_or(crate::model::NO_SELECTIONS, |index| {
        state.reserved_resources[index].amount
    });
    let remaining = budget
        .amount
        .checked_sub(reserved)
        .ok_or(crate::error::CampaignError::BudgetExhausted)?;
    debug_assert!(remaining <= budget.amount);
    Ok(remaining)
}
