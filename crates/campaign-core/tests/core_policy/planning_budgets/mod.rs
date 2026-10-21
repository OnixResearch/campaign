//! Checked multi-dimension budgets (`campaign.budgets`).

mod waits;

const TIME_BUDGET: u64 = 16;
const MEMORY_BUDGET: u64 = 3;
const TIME_COST: u64 = 3;
const MEMORY_COST: u64 = 2;
const MEMORY_LIGHT_COST: u64 = 1;
const RANK_TOP: i64 = 20;
const RANK_BOTTOM: i64 = 10;

fn time() -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::ResourceDimension, b"resource")
}

fn memory() -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::ResourceDimension, b"memory")
}

fn amount(dimension_id: campaign_core::Id, amount: u64) -> campaign_core::ResourceAmount {
    campaign_core::ResourceAmount {
        dimension_id,
        amount,
    }
}

fn two_dimensions() -> Result<crate::suite::planning_support::History, String> {
    crate::suite::planning_support::admitted_with(|input| {
        input.resource_budgets = vec![amount(time(), TIME_BUDGET), amount(memory(), MEMORY_BUDGET)];
    })
}

fn sorted(mut amounts: Vec<campaign_core::ResourceAmount>) -> Vec<campaign_core::ResourceAmount> {
    amounts.sort_by_key(|item| item.dimension_id);
    amounts
}

fn guided_costed(
    history: crate::suite::planning_support::History,
    label: &'static [u8],
    costs: Vec<campaign_core::ResourceAmount>,
    rank: i64,
) -> Result<(crate::suite::planning_support::History, campaign_core::Id), String> {
    let (events, state) = history;
    let (events, state, candidate_id) =
        crate::suite::planning_support::admit_costed(events, &state, label, costs)?;
    let (events, state, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        rank,
        crate::suite::planning_support::FIRST_GENERATION,
    )?;
    Ok(((events, state), candidate_id))
}

#[test]
// r[verify campaign.budgets.valid]
fn costs_that_fit_every_budget_are_reserved_in_every_dimension() -> Result<(), String> {
    let costs = vec![amount(time(), TIME_COST), amount(memory(), MEMORY_COST)];
    let (history, candidate_id) =
        guided_costed(two_dimensions()?, b"both", costs.clone(), RANK_TOP)?;
    let (events, state) = history;
    let (_events, published, selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(selection.candidate_id, candidate_id);
    assert_eq!(selection.reserved_costs, sorted(costs.clone()));
    assert_eq!(published.reserved_resources, sorted(costs));
    assert!(state.reserved_resources.is_empty());
    Ok(())
}

#[test]
// r[verify campaign.budgets.exhausted]
// r[verify campaign.stop.budget]
fn an_exhausted_dimension_blocks_selection_and_is_named() -> Result<(), String> {
    let heavy = vec![amount(time(), TIME_COST), amount(memory(), MEMORY_COST)];
    let (history, heavy_id) = guided_costed(two_dimensions()?, b"heavy", heavy, RANK_TOP)?;
    let (history, light_id) = guided_costed(
        history,
        b"light",
        vec![
            amount(time(), TIME_COST),
            amount(memory(), MEMORY_LIGHT_COST),
        ],
        RANK_BOTTOM,
    )?;
    let (events, state) = history;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(first.candidate_id, heavy_id);
    let (events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &first,
        (b"heavy-child", b"heavy-payload"),
        Vec::new(),
    )?;
    let (events, state, second) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(second.candidate_id, light_id);
    let (_events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &second,
        (b"light-child", b"light-payload"),
        Vec::new(),
    )?;
    // Time (6 of 16 reserved) still fits both candidates; only memory (3 of 3) blocks them.
    let stop =
        crate::suite::planning_support::stopped(&crate::suite::planning_support::ranked(&state)?)?;
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::BudgetExhausted {
            dimensions: vec![memory()],
        }
    );
    Ok(())
}

/// Unrepresentable depth arithmetic still fails, but a saturated budget is exhausted, never an
/// overflow: planning waits and replay refuses the over-budget selection.
#[test]
// r[verify campaign.budgets.overflow]
// r[verify campaign.budgets.fit]
fn a_saturated_budget_is_exhausted_without_overflow_at_selection_and_replay() -> Result<(), String>
{
    let (_events, state) = crate::suite::admission::initial_state()?;
    let deep_parent = crate::suite::admission::xid(campaign_core::IdKind::ChildMoment, b"deep");
    let forged = crate::suite::planning_support::rebuilt(state, |forged| {
        forged.known_moments.push(campaign_core::KnownMoment {
            moment_id: deep_parent,
            depth: u64::MAX,
        });
    })?;
    let mut record =
        crate::suite::candidate::record(&forged, b"deep", crate::suite::constants::COST_ONE);
    record.parent_moment_id = deep_parent;
    assert_eq!(
        crate::suite::planning_support::batch(&forged, vec![record]),
        Err(campaign_core::CampaignError::ArithmeticOverflow)
    );

    let history = crate::suite::planning_support::admitted_with(|input| {
        input.resource_budgets[0].amount = u64::MAX;
    })?;
    let (history, _saturating) = guided_costed(
        history,
        b"saturating",
        vec![amount(time(), u64::MAX)],
        RANK_TOP,
    )?;
    let (history, unit_id) = guided_costed(
        history,
        b"unit",
        vec![amount(time(), crate::suite::constants::COST_ONE)],
        RANK_BOTTOM,
    )?;
    let (events, state) = history;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(
        crate::suite::planning_support::plan_with(
            &state,
            crate::suite::selection::ranked_profile()?,
            None
        )
        .map(|plan| plan.decision),
        Ok(campaign_core::NextDecision::Wait(
            campaign_core::WaitReason::AwaitingResults
        ))
    );
    let unit = crate::suite::planning_support::observed(state.candidate(unit_id))?;
    let unit_guidance = unit
        .latest_guidance
        .clone()
        .ok_or_else(|| String::from("unit must be guided"))?;
    let replayed = crate::suite::planning_support::forged_selection(&first, |decision| {
        decision.state_before_id = state.state_id;
        decision.ordinal = crate::suite::planning_support::SECOND_ORDINAL;
        decision.candidate_id = unit_id;
        decision.guidance_id = unit_guidance.guidance_id;
        decision.reserved_costs = unit.candidate.declared_costs.clone();
    })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, replayed),
        Err(campaign_core::CampaignError::BudgetExhausted)
    );
    Ok(())
}

#[test]
fn an_observed_result_does_not_refund_reserved_costs() -> Result<(), String> {
    let (events, state) = crate::suite::planning_support::admitted_with(|input| {
        input.resource_budgets[0].amount = crate::suite::constants::BUDGET_SMALL;
    })?;
    let (events, state, _only) =
        crate::suite::planning_support::guided(events, &state, b"only", RANK_TOP)?;
    let (events, state, selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let (_events, settled) = crate::suite::planning_support::settle(
        events,
        &state,
        &selection,
        (b"refund-child", b"refund-payload"),
        Vec::new(),
    )?;
    assert!(settled.outstanding.is_empty());
    assert_eq!(settled.reserved_resources, state.reserved_resources);
    let stop = crate::suite::planning_support::stopped(&crate::suite::planning_support::ranked(
        &settled,
    )?)?;
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::BudgetExhausted {
            dimensions: vec![time()],
        }
    );
    Ok(())
}
