//! Total ranking over one rank shape: over more than twenty guided candidates the ranked and
//! exploratory planners agree with the reference order and never panic, and guidance of any other
//! rank-vector length than the campaign's first record is refused at admission, revision, and
//! replay with `InvalidGuidanceShape`.

/// Generated candidate count range: above the length where `slice::sort_by` detects a
/// non-total comparator.
const MIN_CANDIDATES: usize = 21;
const MAX_CANDIDATES: usize = 32;
/// Candidate, guidance, selection, and retention bound of every ranking campaign.
const WIDE: u64 = 64;
/// Control-event bound: the batch, every guidance record, and each round's selection fit.
const CONTROL_EVENTS: u64 = 256;
/// Exclusive rank value bound; small so equal prefixes and full ties are common.
const RANK_SPREAD: i64 = 3;
/// Budget of the single resource dimension; every candidate costs one unit.
const BUDGET: u64 = 1024;
/// Planning rounds per case; each publishes its ranked selection, so decay reorders ranks.
const ROUNDS: u64 = 3;

#[derive(Clone, Debug)]
struct RankFacts {
    ranks: Vec<Vec<i64>>,
    other_dimensions: usize,
    pick: u8,
    exploration: crate::suite::strategies::Exploration,
    draws: crate::suite::requests::Draws,
}

/// The campaign's rank-vector length and one other declared length.
#[derive(Clone, Copy, Debug)]
struct Shapes {
    admitted_dimensions: usize,
    other_dimensions: usize,
}

impl Shapes {
    /// `step_count` in `1..MAX_RANK_DIMENSIONS` steps cyclically away from `admitted_dimensions`.
    const fn of(generated: (usize, usize)) -> Self {
        let (admitted_dimensions, step_count) = generated;
        let other_dimensions = admitted_dimensions
            .saturating_sub(1)
            .saturating_add(step_count)
            .rem_euclid(crate::suite::strategies::MAX_RANK_DIMENSIONS)
            .saturating_add(1);
        Self {
            admitted_dimensions,
            other_dimensions,
        }
    }
}

fn rank_facts() -> impl proptest::strategy::Strategy<Value = RankFacts> {
    let declared = crate::suite::strategies::MAX_RANK_DIMENSIONS;
    proptest::strategy::Strategy::prop_flat_map((1..=declared, 1..declared), |generated| {
        facts_of(Shapes::of(generated))
    })
}

fn facts_of(shapes: Shapes) -> impl proptest::strategy::Strategy<Value = RankFacts> {
    let rank_dimensions = shapes.admitted_dimensions;
    assert_ne!(shapes.other_dimensions, rank_dimensions);
    assert!(shapes.other_dimensions <= crate::suite::strategies::MAX_RANK_DIMENSIONS);
    proptest::strategy::Strategy::prop_map(
        (
            proptest::collection::vec(
                proptest::collection::vec(0..RANK_SPREAD, rank_dimensions),
                MIN_CANDIDATES..=MAX_CANDIDATES,
            ),
            proptest::num::u8::ANY,
            crate::suite::strategies::threshold(),
            crate::suite::strategies::exploration_order(rank_dimensions),
            crate::suite::strategies::draws(),
        ),
        move |(ranks, pick, threshold, order, draws)| RankFacts {
            ranks,
            other_dimensions: shapes.other_dimensions,
            pick,
            exploration: crate::suite::strategies::Exploration { threshold, order },
            draws,
        },
    )
}

fn applied(
    state: campaign_core::FrontierState,
    event: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
) -> Result<campaign_core::FrontierState, String> {
    let event = crate::suite::fixture::observed(event)?;
    crate::suite::fixture::observed(campaign_core::apply_control_event(state, &event))
}

/// Admits one candidate per rank vector in one batch, then guides each with its ranks.
fn guided(
    facts: &RankFacts,
) -> Result<(campaign_core::FrontierState, Vec<campaign_core::Id>), String> {
    let limits = crate::suite::fixture::Limits {
        max_outstanding: WIDE,
        max_candidates: WIDE,
        max_candidate_batch: WIDE,
        max_selections: WIDE,
        max_guidance_records: WIDE,
        max_depth: WIDE,
        max_control_events: CONTROL_EVENTS,
        rank_dimensions: crate::suite::strategies::MAX_RANK_DIMENSIONS,
    };
    let mut bounds = crate::suite::fixture::bounds(&limits)?;
    bounds.bounds_id = campaign_core::Id::absent();
    bounds.max_retained_candidates = WIDE;
    let bounds = crate::suite::fixture::observed(bounds.derive())?;
    let projection = crate::suite::fixture::projection()?;
    let input = crate::suite::fixture::admission(
        bounds,
        vec![crate::suite::fixture::root(&projection, 0)],
        vec![campaign_core::ResourceAmount {
            dimension_id: crate::suite::fixture::dimension(0),
            amount: BUDGET,
        }],
        vec![crate::suite::fixture::schema(0)],
    )?;
    let (_event, state) = crate::suite::fixture::campaign(input)?;
    let root = *state
        .known_moments
        .first()
        .ok_or_else(|| String::from("a campaign has a root"))?;
    let mut records = Vec::with_capacity(facts.ranks.len());
    for label in (0_u64..).take(facts.ranks.len()) {
        let record = crate::suite::fixture::record(
            &state,
            label,
            root,
            crate::suite::fixture::costs(&[campaign_core::UNIT_COST]),
        );
        records.push(crate::suite::fixture::observed(record.derive())?);
    }
    let ids: Vec<campaign_core::Id> = records.iter().map(|record| record.candidate_id).collect();
    let mut state = applied(
        state.clone(),
        campaign_core::admit_candidates(
            &state,
            records,
            crate::suite::fixture::rule(false),
            crate::suite::fixture::coalescing_policy(),
        ),
    )?;
    for (candidate_id, ranks) in ids.iter().zip(&facts.ranks) {
        let guidance = crate::suite::fixture::guidance(
            &state,
            *candidate_id,
            crate::suite::fixture::FIRST_GENERATION,
            ranks.clone(),
        );
        state = applied(
            state.clone(),
            campaign_core::admit_guidance(&state, guidance),
        )?;
    }
    assert_eq!(state.candidates.len(), facts.ranks.len());
    assert_eq!(
        usize::try_from(state.guidance_record_count).ok(),
        Some(facts.ranks.len())
    );
    Ok((state, ids))
}

/// Guidance of another rank-vector length is refused as an update, a revision, and a replayed
/// event, each without producing a state.
fn other_shapes_are_refused(
    facts: &RankFacts,
    state: &campaign_core::FrontierState,
    ids: &[campaign_core::Id],
) -> Result<(), String> {
    let index = crate::suite::fixture::position(facts.pick, ids.len())?;
    let candidate_id = ids[index];
    let latest = state
        .candidate(candidate_id)
        .ok()
        .and_then(|candidate| candidate.latest_guidance.clone())
        .ok_or_else(|| String::from("every candidate is guided"))?;
    let next_generation = crate::suite::fixture::FIRST_GENERATION.saturating_add(1);
    let other = vec![RANK_SPREAD; facts.other_dimensions];
    assert_ne!(other.len(), latest.ranks.len());
    let shape = campaign_core::CampaignError::InvalidGuidanceShape;
    let update = crate::suite::fixture::guidance(state, candidate_id, next_generation, other);
    assert_eq!(
        crate::suite::fixture::denied(campaign_core::admit_guidance(state, update.clone()))?,
        shape
    );
    let revision = campaign_core::GuidanceRevisionRequest {
        candidate_id,
        successor: update.clone(),
        invalidated_guidance_id: latest.guidance_id,
        reason_id: crate::suite::fixture::id(campaign_core::IdKind::Reason, b"reshape", 0),
    };
    assert_eq!(
        crate::suite::fixture::denied(campaign_core::revise_guidance(state, revision))?,
        shape
    );
    let replayed = crate::suite::fixture::observed(
        campaign_core::CampaignControlEvent::GuidanceAdmitted(campaign_core::GuidanceEvent {
            event_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            guidance: update,
            dynamics: campaign_core::GuidanceDynamics::StrictImprovement {
                previous_guidance_id: latest.guidance_id,
            },
            admission_outcomes_id: campaign_core::Id::absent(),
        })
        .canonical(),
    )?;
    assert_eq!(
        crate::suite::fixture::denied(campaign_core::apply_control_event(
            state.clone(),
            &replayed
        ))?,
        shape
    );
    Ok(())
}

const fn selected_candidate(plan: &campaign_core::DecisionPlan) -> Option<campaign_core::Id> {
    match &plan.decision {
        campaign_core::NextDecision::Select(selection) => Some(selection.candidate_id),
        campaign_core::NextDecision::Wait(_) | campaign_core::NextDecision::Stop(_) => None,
    }
}

fn one_shape_is_total(facts: &RankFacts) -> Result<(), String> {
    let (mut state, ids) = guided(facts)?;
    other_shapes_are_refused(facts, &state, &ids)?;
    let ranked = crate::suite::requests::profile(campaign_core::SelectionMode::Ranked)?;
    for round in 0..ROUNDS {
        let explore = crate::suite::requests::exploratory(
            &state,
            facts.exploration.threshold,
            facts.exploration.order.clone(),
        )?;
        let ticket = crate::suite::requests::ticket(&state, &explore, facts.draws, round)?;
        let explore_request = crate::suite::requests::request(&state, explore, Some(ticket));
        let explored = crate::suite::fixture::observed(crate::suite::requests::plan(
            &state,
            explore_request.clone(),
        ))?;
        assert_eq!(
            selected_candidate(&explored),
            crate::suite::oracle::expected_choice(&state, &explore_request)?
        );
        let ranked_request = crate::suite::requests::request(&state, ranked.clone(), None);
        let plan = crate::suite::fixture::observed(crate::suite::requests::plan(
            &state,
            ranked_request.clone(),
        ))?;
        let reference = crate::suite::oracle::ranked_order(&state)?;
        let campaign_core::NextDecision::Select(selection) = plan.decision else {
            return Err(format!(
                "a guided frontier within budget selects, got {plan:?}"
            ));
        };
        assert_eq!(Some(selection.candidate_id), reference.first().copied());
        assert_eq!(
            Some(selection.candidate_id),
            crate::suite::oracle::expected_choice(&state, &ranked_request)?
        );
        let event = campaign_core::SelectionEvent::new(*selection, ranked.clone(), None)
            .map(campaign_core::CampaignControlEvent::SelectionPublished);
        state = applied(state, event)?;
    }
    assert_eq!(state.selection_count, ROUNDS);
    Ok(())
}

#[test]
// r[verify campaign.guidance.shape]
// r[verify campaign.guidance.valid]
fn planning_over_one_rank_shape_is_total_and_matches_the_reference() -> Result<(), String> {
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &rank_facts(),
        |facts| one_shape_is_total(&facts),
    )
}
