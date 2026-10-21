/// A state is current only in canonical order with its re-derived identity, with an admission
/// that re-derives and validates, and with no key repeated in any keyed vector (`StaleState`
/// otherwise).
///
/// The state identity frames the admission as stored, not whether its identities re-derive from
/// its values, so the admission is re-derived here: its campaign identity, its bounds identity,
/// the bound values that identity frames, and its admitted projection binding. A caller-built
/// state whose admission was edited (a bound raised without re-deriving `bounds_id` and
/// `campaign_id`, or the admitted binding changed under an unchanged campaign identity) is
/// refused before any policy reads it. Canonical order alone is a stable sort, which leaves a
/// repeated key in place; the core never produces one, so a caller-built state that repeats a
/// key is refused as well. Other caller-built inconsistencies between the state's counters and
/// records (for example an edited selection count or reserved resources) are not rechecked:
/// in-contract states come from `reconstruct` and `apply_control_event`.
// r[impl campaign.identity.admission_current]
pub(crate) fn assert_state_identity_current(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<()> {
    let canonical = state.clone().with_identity()?;
    if canonical != *state || state.admission.validate().is_err() {
        return Err(crate::error::CampaignError::StaleState);
    }
    assert_keys_strictly_increasing(state)
}
/// Refuses (`StaleState`) a state in which some keyed vector, the admission's and the nested ones
/// included (each candidate's declared costs, each outstanding selection's reserved costs, and a
/// recorded budget stop's dimensions), does not have strictly increasing keys.
// r[impl campaign.cache]
// r[impl campaign.budgets.fit]
pub(crate) fn assert_keys_strictly_increasing(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<()> {
    if !(admission_keys_strictly_increasing(&state.admission)
        && frontier_keys_strictly_increasing(state)
        && nested_keys_strictly_increasing(state))
    {
        return Err(crate::error::CampaignError::StaleState);
    }
    debug_assert!(crate::identity::unique_sorted_ids(&state.event_ids));
    debug_assert!(crate::identity::unique_sorted_ids(&state.accepted_results));
    Ok(())
}
/// Whether every keyed vector nested in the state's candidates, outstanding selections, and
/// recorded stop has strictly increasing keys, as intake and replay derive them.
fn nested_keys_strictly_increasing(state: &crate::frontier::FrontierState) -> bool {
    let by_dimension = |cost: &crate::model::ResourceAmount| cost.dimension_id;
    state.candidates.iter().all(|candidate| {
        crate::identity::collection::strictly_increasing_by_key(
            &candidate.candidate.declared_costs,
            by_dimension,
        )
    }) && state.outstanding.iter().all(|selection| {
        crate::identity::collection::strictly_increasing_by_key(
            &selection.reserved_costs,
            by_dimension,
        )
    }) && state.stopped.as_ref().is_none_or(|stop| {
        let crate::model::StopReason::BudgetExhausted { dimensions } = &stop.reason else {
            return true;
        };
        crate::identity::collection::strictly_increasing_by_key(dimensions, |id| *id)
    })
}
fn admission_keys_strictly_increasing(admission: &crate::model::CampaignAdmission) -> bool {
    crate::identity::collection::strictly_increasing_by_key(&admission.roots, |root| root.moment_id)
        && crate::identity::collection::strictly_increasing_by_key(
            &admission.resource_budgets,
            |budget| budget.dimension_id,
        )
        && crate::identity::collection::strictly_increasing_by_key(
            &admission.operation_schema_ids,
            |schema_id| *schema_id,
        )
}
fn frontier_keys_strictly_increasing(state: &crate::frontier::FrontierState) -> bool {
    crate::identity::collection::strictly_increasing_by_key(&state.candidates, |candidate| {
        candidate.candidate.candidate_id
    }) && crate::identity::collection::strictly_increasing_by_key(&state.outstanding, |selection| {
        selection.selection_id
    }) && crate::identity::collection::strictly_increasing_by_key(&state.accepted_results, |id| *id)
        && crate::identity::collection::strictly_increasing_by_key(
            &state.discoveries,
            |discovery| discovery.child_moment_id,
        )
        && crate::identity::collection::strictly_increasing_by_key(&state.known_moments, |moment| {
            moment.moment_id
        })
        && crate::identity::collection::strictly_increasing_by_key(&state.settled, |settlement| {
            settlement.selection_id
        })
        && crate::identity::collection::strictly_increasing_by_key(&state.reservations, |id| *id)
        && crate::identity::collection::strictly_increasing_by_key(
            &state.payload_bindings,
            |binding| binding.payload_id,
        )
        && crate::identity::collection::strictly_increasing_by_key(&state.findings, |finding| {
            finding.finding_id
        })
        && crate::identity::collection::strictly_increasing_by_key(&state.event_ids, |id| *id)
        && crate::identity::collection::strictly_increasing_by_key(
            &state.reserved_resources,
            |resource| resource.dimension_id,
        )
}
/// New work (candidates, guidance, revisions, retirements, prunes) needs an open campaign and
/// control-event headroom for the event, every outstanding settlement, and a stop.
// r[impl campaign.budgets.control_event_reserve]
// r[impl campaign.stop.terminal]
pub(super) fn ensure_open_for_new_work(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<()> {
    if state.stopped.is_some() {
        return Err(crate::error::CampaignError::CampaignStopped);
    }
    crate::frontier::reserve::ensure_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::NEW_WORK,
    )
}
/// Checks declared rank-dimension indices (an exploration or retention order) against the rank
/// shape: every index lies below `max_rank_dimensions` and, once the first guidance record has
/// fixed the campaign's rank-vector length, below that length (`InvalidConfig` otherwise).
///
/// Every later guidance record must keep the fixed length, so an order naming a dimension beyond
/// it can never be served; refusing it here, before any entropy draw is consulted, keeps the
/// planner's answer independent of the draw.
// r[impl campaign.guidance.declared_order]
pub(super) fn validate_declared_rank_dimensions(
    state: &crate::frontier::FrontierState,
    dimensions: &[u32],
) -> crate::error::CampaignResult<()> {
    let bound = state.admission.bounds.max_rank_dimensions;
    let dimension_count = match crate::frontier::validation::guidance_rank_dimensions(state) {
        Some(shape) => crate::identity::usize_to_u64(shape)?.min(bound),
        None => bound,
    };
    if dimensions
        .iter()
        .any(|index| u64::from(*index) >= dimension_count)
    {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    debug_assert!(dimension_count <= bound);
    Ok(())
}
