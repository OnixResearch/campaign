//! D90: the keyed vectors nested in a caller-built state (each candidate's declared costs, each
//! outstanding selection's reserved costs, and a recorded budget stop's dimensions) must have
//! strictly increasing keys as well; a repeated or descending key is `StaleState` before any
//! policy reads it.

fn amount(dimension_id: campaign_core::Id, amount: u64) -> campaign_core::ResourceAmount {
    campaign_core::ResourceAmount {
        dimension_id,
        amount,
    }
}

/// A dimension identity ordered after `dimension_id` that no budget declares.
fn later_dimension(dimension_id: campaign_core::Id) -> Result<campaign_core::Id, String> {
    [b"nested-a".as_slice(), b"nested-b", b"nested-c"]
        .into_iter()
        .map(|label| crate::suite::admission::xid(campaign_core::IdKind::ResourceDimension, label))
        .find(|candidate| *candidate > dimension_id)
        .ok_or_else(|| String::from("no later dimension identity"))
}

/// The round-9 reproducer: under a budget of `BUDGET_TOTAL`, a caller-built candidate that
/// declares its one budget dimension twice (5 + 5) used to receive a selection reserving 10,
/// which replay accepted. Now planning, stop admission, and replay of the honest selection all
/// refuse the state as `StaleState`, as they do for a descending pair of dimensions. The same
/// candidate declaring the dimension once is planned, reserves exactly its cost, and applies.
#[test]
// r[verify campaign.cache]
// r[verify campaign.budgets.fit]
fn a_candidate_declaring_a_dimension_twice_or_out_of_order_is_stale() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (_events, state, _id) = crate::suite::planning_support::guided(
        events,
        &state,
        b"nested-costs",
        crate::suite::planning_support::RANK_TOP,
    )?;
    let dimension = state.admission.resource_budgets[0].dimension_id;
    assert_eq!(
        state.admission.resource_budgets[0].amount,
        crate::suite::constants::BUDGET_TOTAL
    );
    let once = crate::suite::planning_support::rebuilt(state.clone(), |state| {
        state.candidates[0].candidate.declared_costs = vec![amount(dimension, 5)];
    })?;
    let selection =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&once)?)?;
    assert_eq!(selection.reserved_costs, vec![amount(dimension, 5)]);
    let event = crate::suite::selection::event(selection)?;
    let next =
        crate::suite::planning_support::observed(campaign_core::apply_control_event(once, &event))?;
    assert_eq!(next.reserved_resources, vec![amount(dimension, 5)]);
    let later = later_dimension(dimension)?;
    for costs in [
        vec![amount(dimension, 5), amount(dimension, 5)],
        vec![amount(later, 1), amount(dimension, 5)],
    ] {
        let forged = crate::suite::planning_support::rebuilt(state.clone(), |state| {
            state.candidates[0].candidate.declared_costs = costs.clone();
        })?;
        let refused = Err(campaign_core::CampaignError::StaleState);
        assert_eq!(
            crate::suite::planning_support::plan_with(
                &forged,
                crate::suite::selection::ranked_profile()?,
                None
            )
            .map(|_| ()),
            refused,
            "{costs:?}"
        );
        assert_eq!(
            campaign_core::admit_stop(&forged, campaign_core::StopReason::FrontierExhausted)
                .map(|_| ()),
            refused,
            "{costs:?}"
        );
        assert_eq!(
            campaign_core::apply_control_event(forged, &event).map(|_| ()),
            refused,
            "{costs:?}"
        );
    }
    Ok(())
}

/// An outstanding selection whose reserved costs repeat or descend, and a recorded budget stop
/// whose dimensions repeat or descend, make the state `StaleState` at intake, at replay, and at
/// retention planning. With strictly increasing keys the same forges are current: the pending
/// result is admitted and applies, and retention planning prunes nothing (open) or answers
/// `CampaignStopped` (stopped).
#[test]
// r[verify campaign.cache]
// r[verify campaign.budgets.fit]
fn repeated_or_descending_reserved_costs_or_stop_dimensions_are_stale() -> Result<(), String> {
    let (_events, state, pending) = super::populated()?;
    let dimension = state.admission.resource_budgets[0].dimension_id;
    let later = later_dimension(dimension)?;
    let refused = Err(campaign_core::CampaignError::StaleState);
    for (label, keys, is_stale) in [
        ("repeated", [dimension, dimension], true),
        ("descending", [later, dimension], true),
        ("increasing", [dimension, later], false),
    ] {
        let reserved = crate::suite::planning_support::rebuilt(state.clone(), |state| {
            state.outstanding[0].reserved_costs = keys.iter().map(|key| amount(*key, 1)).collect();
        })?;
        let stopped = crate::suite::planning_support::rebuilt(state.clone(), |state| {
            state.stopped = Some(campaign_core::StopDecision {
                stop_id: crate::suite::admission::xid(campaign_core::IdKind::Stop, b"nested"),
                campaign_id: state.admission.campaign_id,
                state_before_id: state.state_id,
                reason: campaign_core::StopReason::BudgetExhausted {
                    dimensions: keys.to_vec(),
                },
                incomplete_search_acknowledged: true,
            });
        })?;
        for (vector, forged) in [("reserved_costs", reserved), ("dimensions", stopped)] {
            let super::Answers {
                admitted,
                applied,
                planned,
            } = super::answers(&forged, &pending)?;
            if is_stale {
                assert_eq!(admitted.map(|_| ()), refused, "{label} {vector}");
                assert_eq!(applied.map(|_| ()), refused, "{label} {vector}");
                assert_eq!(planned.map(|_| ()), refused, "{label} {vector}");
            } else {
                let retained = if vector == "dimensions" {
                    Err(campaign_core::CampaignError::CampaignStopped)
                } else {
                    Ok(None)
                };
                assert_eq!(admitted.map(|_| ()), Ok(()), "{label} {vector}");
                assert_eq!(applied.map(|_| ()), Ok(()), "{label} {vector}");
                assert_eq!(planned, retained, "{label} {vector}");
            }
        }
    }
    Ok(())
}
