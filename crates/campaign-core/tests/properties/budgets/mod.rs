//! Budget monotonicity over generated selections and settlements: reserved resources never
//! decrease, never exceed their budgets, and equal the costs of every published selection.

pub(crate) fn reserved_totals(
    state: &campaign_core::FrontierState,
    selections: &[campaign_core::SelectionDecision],
) -> Vec<(campaign_core::Id, u64, u64)> {
    state
        .admission
        .resource_budgets
        .iter()
        .map(|budget| {
            let published: u64 = selections
                .iter()
                .filter(|selection| selection.ordinal <= state.selection_count)
                .flat_map(|selection| selection.reserved_costs.iter())
                .filter(|cost| cost.dimension_id == budget.dimension_id)
                .map(|cost| cost.amount)
                .sum();
            (budget.dimension_id, budget.amount, published)
        })
        .collect()
}

fn reservations_are_monotone(history: &crate::suite::interpreter::History) -> Result<(), String> {
    let mut previous: Option<&campaign_core::FrontierState> = None;
    for state in &history.checkpoints {
        assert!(state.reserved_resources.iter().all(|reserved| {
            state
                .admission
                .resource_budgets
                .iter()
                .any(|budget| budget.dimension_id == reserved.dimension_id)
        }));
        for (dimension_id, budget, published) in reserved_totals(state, &history.selections) {
            let reserved = crate::suite::oracle::reserved(state, dimension_id);
            assert!(
                reserved <= budget,
                "reserved {reserved} exceeds budget {budget}"
            );
            assert_eq!(
                reserved, published,
                "reservations equal published costs, no refund"
            );
            if let Some(before) = previous {
                assert!(crate::suite::oracle::reserved(before, dimension_id) <= reserved);
            }
        }
        previous = Some(state);
    }
    for selection in &history.selections {
        let declared = history
            .checkpoints
            .iter()
            .find(|state| state.state_id == selection.state_before_id)
            .ok_or_else(|| String::from("a selection binds a reconstructed state"))
            .and_then(|state| {
                crate::suite::fixture::observed(state.candidate(selection.candidate_id))
            })?
            .candidate
            .declared_costs
            .clone();
        assert_eq!(selection.reserved_costs, declared);
    }
    Ok(())
}

#[test]
// r[verify campaign.budgets.no_refund]
// r[verify campaign.budgets.valid]
fn reserved_resources_only_grow_within_declared_amounts() -> Result<(), String> {
    let settled = core::cell::Cell::new(0_u64);
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &crate::suite::strategies::script(),
        |script| {
            let history = crate::suite::interpreter::run(&script)?;
            let state = history.state()?;
            let results =
                u64::try_from(state.accepted_results.len()).map_err(|error| format!("{error}"))?;
            assert!(results <= state.selection_count);
            settled.set(settled.get().saturating_add(results));
            reservations_are_monotone(&history)
        },
    )?;
    assert!(
        settled.get() > 0,
        "no generated history settled a selection"
    );
    Ok(())
}
