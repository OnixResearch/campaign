//! Reference budget fit: the documented D36 rule restated over the public model. One more
//! reservation of a cost fits exactly when the cost is at most the budget left after the reserved
//! amount (`cost <= budget - reserved`); a remainder that is not representable (the reserved
//! amount is above the budget or unrepresentable) does not fit.

/// Dimensions in which one more reservation of the candidate would exceed its budget.
///
/// # Errors
///
/// Returns `BudgetExhausted` for a declared cost in a dimension without a budget.
pub(super) fn blocking_dimensions(
    state: &campaign_core::FrontierState,
    candidate: &campaign_core::CandidateState,
) -> campaign_core::CampaignResult<alloc::vec::Vec<campaign_core::Id>> {
    let mut over = alloc::vec::Vec::with_capacity(candidate.candidate.declared_costs.len());
    for cost in &candidate.candidate.declared_costs {
        let budget = state
            .admission
            .resource_budgets
            .iter()
            .find(|budget| budget.dimension_id == cost.dimension_id)
            .ok_or(campaign_core::CampaignError::BudgetExhausted)?;
        let reserved = state
            .reserved_resources
            .iter()
            .filter(|reserved| reserved.dimension_id == cost.dimension_id)
            .try_fold(0_u64, |sum, reserved| sum.checked_add(reserved.amount));
        let remaining = reserved.and_then(|reserved| budget.amount.checked_sub(reserved));
        let is_within_budget = remaining.is_some_and(|remaining| cost.amount <= remaining);
        if !is_within_budget {
            over.push(cost.dimension_id);
        }
    }
    debug_assert!(over.len() <= candidate.candidate.declared_costs.len());
    debug_assert!(over.iter().all(|dimension| {
        candidate
            .candidate
            .declared_costs
            .iter()
            .any(|cost| cost.dimension_id == *dimension)
    }));
    Ok(over)
}

/// Budget dimensions that block some retained candidate, guided or not, ascending.
///
/// # Errors
///
/// Returns `BudgetExhausted` for a declared cost in a dimension without a budget.
pub(super) fn exhausted_dimensions(
    state: &campaign_core::FrontierState,
) -> campaign_core::CampaignResult<alloc::vec::Vec<campaign_core::Id>> {
    let mut dimensions = alloc::vec::Vec::with_capacity(state.admission.resource_budgets.len());
    for candidate in &state.candidates {
        if candidate.status == campaign_core::CandidateStatus::Retained {
            dimensions.extend(blocking_dimensions(state, candidate)?);
        }
    }
    dimensions.sort();
    dimensions.dedup();
    debug_assert!(dimensions.len() <= state.admission.resource_budgets.len());
    Ok(dimensions)
}

/// Whether guidance could still make a candidate selectable: one more guidance record fits
/// `max_guidance_records` and some retained candidate without guidance has declared costs that
/// fit the remaining budgets. The planner awaits it only while the control events also admit
/// its guidance and one selection.
///
/// # Errors
///
/// Returns `BudgetExhausted` for a declared cost in a dimension without a budget.
pub(super) fn has_guidable_candidate(
    state: &campaign_core::FrontierState,
) -> campaign_core::CampaignResult<bool> {
    if state.guidance_record_count >= state.admission.bounds.max_guidance_records {
        return Ok(false);
    }
    for candidate in &state.candidates {
        let is_unguided_retained = candidate.status == campaign_core::CandidateStatus::Retained
            && candidate.latest_guidance.is_none();
        if is_unguided_retained && blocking_dimensions(state, candidate)?.is_empty() {
            return Ok(true);
        }
    }
    debug_assert!(state.guidance_record_count < state.admission.bounds.max_guidance_records);
    Ok(false)
}
