//! Canonical ordering: planner choices are invariant under candidate batch order and guidance
//! admission order, for ranked and exploratory profiles, before and after one selection.

/// Generated candidate count range.
const MIN_CANDIDATES: usize = 2;
const MAX_CANDIDATES: usize = 6;
/// Budget per dimension; large enough that every candidate is admitted and fits twice.
const ORDER_BUDGET: u64 = 64;
/// Resource dimensions of the ordering campaign.
const ORDER_DIMENSIONS: usize = 2;
/// Planning rounds compared per order (the second follows one publication).
const ROUNDS: u64 = 2;

#[derive(Clone, Debug)]
struct Candidate {
    costs: Vec<u64>,
    ranks: Vec<i64>,
}

#[derive(Clone, Debug)]
struct OrderFacts {
    rank_dimensions: usize,
    candidates: Vec<Candidate>,
    first_keys: Vec<u32>,
    second_keys: Vec<u32>,
    exploration: crate::suite::strategies::Exploration,
    draws: crate::suite::requests::Draws,
}

fn order_facts() -> impl proptest::strategy::Strategy<Value = OrderFacts> {
    proptest::strategy::Strategy::prop_flat_map(
        1..=crate::suite::strategies::MAX_RANK_DIMENSIONS,
        |rank_dimensions| {
            let candidate = proptest::strategy::Strategy::prop_map(
                (
                    proptest::collection::vec(
                        1..=crate::suite::strategies::MAX_COST,
                        ORDER_DIMENSIONS,
                    ),
                    crate::suite::strategies::ranks(rank_dimensions),
                ),
                |(costs, ranks)| Candidate { costs, ranks },
            );
            proptest::strategy::Strategy::prop_map(
                (
                    proptest::collection::vec(candidate, MIN_CANDIDATES..=MAX_CANDIDATES),
                    crate::suite::strategies::keys(),
                    crate::suite::strategies::keys(),
                    crate::suite::strategies::threshold(),
                    crate::suite::strategies::exploration_order(rank_dimensions),
                    crate::suite::strategies::draws(),
                ),
                move |(candidates, first_keys, second_keys, threshold, order, draws)| OrderFacts {
                    rank_dimensions,
                    candidates,
                    first_keys,
                    second_keys,
                    exploration: crate::suite::strategies::Exploration { threshold, order },
                    draws,
                },
            )
        },
    )
}

type Events = Vec<campaign_core::CampaignControlEvent>;

fn push(
    mut events: Events,
    event: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
) -> Result<(Events, campaign_core::FrontierState), String> {
    events.push(crate::suite::fixture::observed(event)?);
    let state = crate::suite::fixture::observed(campaign_core::reconstruct(&events))?;
    Ok((events, state))
}

/// Admits every candidate in one batch in `keys` order, then guides them in `keys` order.
fn guided(
    facts: &OrderFacts,
    keys: &[u32],
) -> Result<(Events, campaign_core::FrontierState), String> {
    let limits = crate::suite::fixture::Limits {
        max_outstanding: crate::suite::fixture::BOUND_WIDE,
        max_candidates: crate::suite::fixture::BOUND_WIDE,
        max_candidate_batch: crate::suite::fixture::BOUND_WIDE,
        max_selections: crate::suite::fixture::BOUND_WIDE,
        max_guidance_records: crate::suite::fixture::BOUND_WIDE,
        max_depth: crate::suite::fixture::BOUND_WIDE,
        max_control_events: crate::suite::fixture::BOUND_WIDE,
        rank_dimensions: facts.rank_dimensions,
    };
    let projection = crate::suite::fixture::projection()?;
    let budgets = (0_u64..)
        .take(ORDER_DIMENSIONS)
        .map(|index| campaign_core::ResourceAmount {
            dimension_id: crate::suite::fixture::dimension(index),
            amount: ORDER_BUDGET,
        })
        .collect();
    let input = crate::suite::fixture::admission(
        crate::suite::fixture::bounds(&limits)?,
        vec![crate::suite::fixture::root(&projection, 0)],
        budgets,
        vec![crate::suite::fixture::schema(0)],
    )?;
    let (event, state) = crate::suite::fixture::campaign(input)?;
    let root = *state
        .known_moments
        .first()
        .ok_or_else(|| String::from("a campaign has a root"))?;
    let mut records = Vec::with_capacity(facts.candidates.len());
    for (label, candidate) in (0_u64..).zip(&facts.candidates) {
        let costs = (0_u64..)
            .zip(&candidate.costs)
            .map(|(index, amount)| campaign_core::ResourceAmount {
                dimension_id: crate::suite::fixture::dimension(index),
                amount: *amount,
            })
            .collect();
        let record = crate::suite::fixture::record(&state, label, root, costs);
        let record = crate::suite::fixture::observed(record.derive())?;
        records.push((record, candidate.ranks.clone()));
    }
    let ordered = crate::suite::strategies::permute(&records, keys);
    let batch = ordered.iter().map(|(record, _)| record.clone()).collect();
    let (mut events, mut state) = push(
        vec![event],
        campaign_core::admit_candidates(
            &state,
            batch,
            crate::suite::fixture::rule(false),
            crate::suite::fixture::coalescing_policy(),
        ),
    )?;
    for (record, ranks) in ordered {
        let guidance = crate::suite::fixture::guidance(
            &state,
            record.candidate_id,
            crate::suite::fixture::FIRST_GENERATION,
            ranks,
        );
        (events, state) = push(events, campaign_core::admit_guidance(&state, guidance))?;
    }
    assert_eq!(state.candidates.len(), facts.candidates.len());
    assert_eq!(
        usize::try_from(state.guidance_record_count).ok(),
        Some(facts.candidates.len())
    );
    Ok((events, state))
}

/// Order-independent facts of one planned decision.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Choice {
    kind: crate::suite::oracle::Kind,
    candidate_id: Option<campaign_core::Id>,
    guidance_id: Option<campaign_core::Id>,
    reserved_costs: Vec<campaign_core::ResourceAmount>,
    ordinal: Option<u64>,
    work: campaign_core::PlanningWork,
}

pub(crate) fn choice(plan: &campaign_core::DecisionPlan) -> Choice {
    let selection = match &plan.decision {
        campaign_core::NextDecision::Select(selection) => Some(selection),
        _ => None,
    };
    Choice {
        kind: crate::suite::oracle::kind(&plan.decision),
        candidate_id: selection.map(|item| item.candidate_id),
        guidance_id: selection.map(|item| item.guidance_id),
        reserved_costs: selection
            .map(|item| item.reserved_costs.clone())
            .unwrap_or_default(),
        ordinal: selection.map(|item| item.ordinal),
        work: plan.work,
    }
}

/// Plans ranked and exploratory choices for `ROUNDS` rounds, publishing each ranked selection.
fn choices(
    facts: &OrderFacts,
    history: (Events, campaign_core::FrontierState),
) -> Result<Vec<Choice>, String> {
    let (mut events, mut state) = history;
    let rounds = usize::try_from(ROUNDS).map_err(|error| format!("{error}"))?;
    let mut out = Vec::with_capacity(rounds.saturating_mul(2));
    for round in 0..ROUNDS {
        let profile = crate::suite::requests::exploratory(
            &state,
            facts.exploration.threshold,
            facts.exploration.order.clone(),
        )?;
        let ticket = crate::suite::requests::ticket(&state, &profile, facts.draws, round)?;
        let explored = crate::suite::fixture::observed(crate::suite::requests::plan(
            &state,
            crate::suite::requests::request(&state, profile, Some(ticket)),
        ))?;
        out.push(choice(&explored));
        let ranked = crate::suite::fixture::observed(crate::suite::requests::plan(
            &state,
            crate::suite::requests::request(
                &state,
                crate::suite::requests::profile(campaign_core::SelectionMode::Ranked)?,
                None,
            ),
        ))?;
        out.push(choice(&ranked));
        let campaign_core::NextDecision::Select(selection) = ranked.decision else {
            return Err(format!(
                "a guided frontier within budget selects, got {ranked:?}"
            ));
        };
        let event = campaign_core::SelectionEvent::new(
            *selection,
            crate::suite::requests::profile(campaign_core::SelectionMode::Ranked)?,
            None,
        )
        .map(campaign_core::CampaignControlEvent::SelectionPublished);
        let before = state.selection_count;
        (events, state) = push(events, event)?;
        assert_eq!(state.selection_count, before.saturating_add(1));
    }
    assert_eq!(out.len(), rounds.saturating_mul(2));
    Ok(out)
}

fn order_is_canonical(facts: &OrderFacts) -> Result<(), String> {
    let first = guided(facts, &facts.first_keys)?;
    let second = guided(facts, &facts.second_keys)?;
    let batch = |history: &(Events, campaign_core::FrontierState)| history.0.get(1).cloned();
    assert_eq!(batch(&first), batch(&second));
    assert_eq!(first.1.candidates, second.1.candidates);
    assert_eq!(first.1.reserved_resources, second.1.reserved_resources);
    assert_eq!(
        first.1.guidance_record_count,
        second.1.guidance_record_count
    );
    let first_choices = choices(facts, first)?;
    let second_choices = choices(facts, second)?;
    assert_eq!(first_choices, second_choices);
    assert!(first_choices.iter().all(|item| item.candidate_id.is_some()));
    Ok(())
}

#[test]
// r[verify campaign.guidance.valid]
fn planner_choices_ignore_batch_and_guidance_admission_order() -> Result<(), String> {
    crate::suite::config::check(crate::suite::config::INPUT_CASES, &order_facts(), |facts| {
        order_is_canonical(&facts)
    })
}
