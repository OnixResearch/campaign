//! Declared exploration orders against the campaign's fixed rank shape (D58): once the first
//! guidance record fixes the rank-vector length, an order naming a dimension at or beyond it is
//! refused with `InvalidConfig` before any entropy draw is consulted, in planning and in replay.

const SINGLE_DIMENSION: u32 = 0;
const BEYOND_SHAPE: u32 = 1;
const THRESHOLD: u64 = 500_000;
const EXPLORE_DRAW: u64 = 0;
const EXPLOIT_DRAW: u64 = 999_999;
const INDEX_DRAW: u64 = 1;
const DRAW_BYTES: usize = 8;
const RANK_FIRST: i64 = 20;
const RANK_SECOND: i64 = 10;

struct Guided {
    events: Vec<campaign_core::CampaignControlEvent>,
    state: campaign_core::FrontierState,
    first: campaign_core::Id,
    second: campaign_core::Id,
}

/// Two candidates guided with one-dimension ranks, which fixes the campaign's rank shape at one.
fn guided() -> Result<Guided, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, first) = crate::suite::candidate::admit(events, &state, b"first", cost)?;
    let (events, state, second) = crate::suite::candidate::admit(events, &state, b"second", cost)?;
    let (events, state, _guidance) =
        crate::suite::candidate::guide(events, &state, first, RANK_FIRST, cost)?;
    let (events, state, _guidance) =
        crate::suite::candidate::guide(events, &state, second, RANK_SECOND, cost)?;
    assert!(u64::from(BEYOND_SHAPE) < state.admission.bounds.max_rank_dimensions);
    Ok(Guided {
        events,
        state,
        first,
        second,
    })
}

fn exploring(
    state: &campaign_core::FrontierState,
    exploration_order: Vec<u32>,
) -> Result<campaign_core::SelectionProfile, String> {
    let entropy_source_id = state.admission.entropy_profile_id;
    crate::suite::schedules::variant(|profile| {
        profile.decay = campaign_core::SelectionCountDecay::None;
        profile.mode = campaign_core::SelectionMode::Exploratory {
            threshold_parts_per_million: THRESHOLD,
            entropy_source_id,
            exploration_order,
        };
    })
}

fn ticket(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    explore_draw: u64,
) -> Result<campaign_core::EntropyTicket, String> {
    let mut ticket = crate::suite::schedules::ticket(state, profile, INDEX_DRAW)?;
    ticket.bytes[..DRAW_BYTES].copy_from_slice(&explore_draw.to_le_bytes());
    ticket.entropy_id = campaign_core::Id::absent();
    let ticket = crate::suite::world::observed(ticket.derive())?;
    assert_eq!(ticket.explore_draw(), explore_draw);
    Ok(ticket)
}

/// Plans `state` under an exploratory profile over `order` with the given explore draw.
fn plan(
    state: &campaign_core::FrontierState,
    order: Vec<u32>,
    explore_draw: u64,
) -> Result<campaign_core::CampaignResult<campaign_core::DecisionPlan>, String> {
    let profile = exploring(state, order)?;
    let entropy = ticket(state, &profile, explore_draw)?;
    Ok(campaign_core::plan_next(
        state,
        crate::suite::selection::request(state, profile, Some(entropy)),
        &state.admission.projection,
    ))
}

#[test]
// r[verify campaign.guidance.declared_order]
fn an_exploration_order_beyond_the_fixed_rank_shape_is_refused_for_every_draw() -> Result<(), String>
{
    let guided = guided()?;
    let state = &guided.state;
    for draw in [EXPLORE_DRAW, EXPLOIT_DRAW] {
        assert_eq!(
            plan(state, vec![BEYOND_SHAPE], draw)?,
            Err(campaign_core::CampaignError::InvalidConfig)
        );
        assert_eq!(
            plan(state, vec![SINGLE_DIMENSION, BEYOND_SHAPE], draw)?,
            Err(campaign_core::CampaignError::InvalidConfig)
        );
    }
    // Within the shape, the explore draw indexes the declared order (first, second) at 1 % 2 and
    // the exploit draw takes the ranked first candidate.
    let explored =
        crate::suite::world::observed(plan(state, vec![SINGLE_DIMENSION], EXPLORE_DRAW)?)?;
    assert_eq!(crate::suite::schedules::selected(&explored)?, guided.second);
    let exploited =
        crate::suite::world::observed(plan(state, vec![SINGLE_DIMENSION], EXPLOIT_DRAW)?)?;
    assert_eq!(crate::suite::schedules::selected(&exploited)?, guided.first);
    // No admissible guidance can widen the fixed shape to serve the declared order.
    let longer = campaign_core::admit_guidance(
        state,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            candidate_id: guided.second,
            generation: crate::suite::constants::GENERATION_NEXT,
            ranks: vec![RANK_FIRST, RANK_FIRST],
            source_facts_id: crate::suite::admission::xid(
                campaign_core::IdKind::SourceFacts,
                b"rank-shape-longer",
            ),
            ordering_profile_id: state.admission.guidance_profile_id,
            adapter_id: state.admission.adapter_id,
        },
    );
    assert_eq!(
        longer.map(|_| ()),
        Err(campaign_core::CampaignError::InvalidGuidanceShape)
    );
    Ok(())
}

#[test]
// r[verify campaign.guidance.declared_order]
fn an_order_within_the_rank_bound_is_refused_once_guidance_fixes_a_shorter_shape()
-> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, &state, b"unguided", cost)?;
    let unfixed = crate::suite::world::observed(plan(&state, vec![BEYOND_SHAPE], EXPLORE_DRAW)?)?;
    assert_eq!(
        unfixed.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    let (_events, fixed, _guidance) =
        crate::suite::candidate::guide(events, &state, candidate_id, RANK_FIRST, cost)?;
    for draw in [EXPLORE_DRAW, EXPLOIT_DRAW] {
        assert_eq!(
            plan(&fixed, vec![BEYOND_SHAPE], draw)?,
            Err(campaign_core::CampaignError::InvalidConfig)
        );
    }
    let within =
        crate::suite::world::observed(plan(&fixed, vec![SINGLE_DIMENSION], EXPLORE_DRAW)?)?;
    assert_eq!(crate::suite::schedules::selected(&within)?, candidate_id);
    Ok(())
}

#[test]
// r[verify campaign.guidance.declared_order]
fn a_replayed_selection_declaring_an_order_beyond_the_rank_shape_is_refused() -> Result<(), String>
{
    let guided = guided()?;
    let state = &guided.state;
    for draw in [EXPLORE_DRAW, EXPLOIT_DRAW] {
        let profile = exploring(state, vec![SINGLE_DIMENSION])?;
        let entropy = ticket(state, &profile, draw)?;
        let genuine = crate::suite::world::observed(campaign_core::plan_next(
            state,
            crate::suite::selection::request(state, profile.clone(), Some(entropy.clone())),
            &state.admission.projection,
        ))?;
        let campaign_core::NextDecision::Select(decision) = genuine.decision else {
            return Err(format!("expected a selection, got {:?}", genuine.decision));
        };
        let mut honest = guided.events.clone();
        honest.push(crate::suite::selection::event_with(
            (*decision).clone(),
            profile,
            Some(entropy),
        )?);
        let replayed = crate::suite::world::observed(campaign_core::reconstruct(&honest))?;
        assert_eq!(replayed.outstanding.len(), 1);
        let beyond = exploring(state, vec![BEYOND_SHAPE])?;
        let beyond_entropy = ticket(state, &beyond, draw)?;
        let forged_decision = crate::suite::world::observed(
            campaign_core::SelectionDecision {
                profile_id: beyond.profile_id,
                entropy_id: Some(beyond_entropy.entropy_id),
                ..*decision
            }
            .derive(),
        )?;
        let mut forged = guided.events.clone();
        forged.push(crate::suite::selection::event_with(
            forged_decision,
            beyond,
            Some(beyond_entropy),
        )?);
        assert_eq!(
            campaign_core::reconstruct(&forged).map(|_| ()),
            Err(campaign_core::CampaignError::InvalidConfig)
        );
    }
    Ok(())
}

/// `campaign.guidance.declared_order`: selection planning validates the profile before it consults
/// the recorded stop, so a stopped campaign refuses an out-of-shape order with `InvalidConfig` and
/// returns its stop for an in-shape one; receipt and scheduling re-planning refuse the order too,
/// but first apply their own history and plan-identity checks (`CrossedIdentity`).
#[test]
// r[verify campaign.guidance.declared_order]
fn a_stopped_campaign_refuses_an_out_of_shape_order_before_returning_its_stop() -> Result<(), String>
{
    let guided = guided()?;
    let stop = crate::suite::world::observed(campaign_core::admit_stop(
        &guided.state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"order-stop"),
        },
    ))?;
    let (_events, stopped) = crate::suite::s3_fixture::append(guided.events.clone(), stop)?;
    let binding = crate::suite::admission::current_projection(&stopped)?;
    let request = |order: Vec<u32>| -> Result<campaign_core::SelectionRequest, String> {
        let profile = exploring(&stopped, order)?;
        let entropy = ticket(&stopped, &profile, EXPLORE_DRAW)?;
        Ok(crate::suite::selection::request_on(
            &stopped,
            profile,
            Some(entropy),
            &binding,
        ))
    };
    let (within, beyond) = (
        request(vec![SINGLE_DIMENSION])?,
        request(vec![BEYOND_SHAPE])?,
    );
    let recorded = crate::suite::world::observed(campaign_core::plan_next(
        &stopped,
        within.clone(),
        &binding,
    ))?;
    assert_eq!(
        crate::suite::planning_support::stopped(&recorded)?,
        stopped.stopped.clone().ok_or("the stop is recorded")?
    );
    let config = Err(campaign_core::CampaignError::InvalidConfig);
    let crossed = Err(campaign_core::CampaignError::CrossedIdentity);
    let planned = campaign_core::plan_next(&stopped, beyond.clone(), &binding);
    assert_eq!(planned.map(|_| ()), config);
    let receipt = |binding: &campaign_core::ProjectionBinding| {
        campaign_core::project_receipt(&stopped, &recorded, &beyond, binding)
    };
    assert_eq!(receipt(&binding).map(|_| ()), config);
    assert_eq!(receipt(&stopped.admission.projection).map(|_| ()), crossed);
    let live = plan(&guided.state, vec![SINGLE_DIMENSION], EXPLORE_DRAW)?;
    let live = crate::suite::world::observed(live)?;
    for (plan, expected) in [(&recorded, config), (&live, crossed)] {
        let explained = campaign_core::explain_selection_difference(
            &stopped,
            &binding,
            campaign_core::PlannedRequest {
                plan: &recorded,
                request: &within,
            },
            campaign_core::PlannedRequest {
                plan,
                request: &beyond,
            },
        );
        assert_eq!(explained.map(|_| ()), expected);
    }
    Ok(())
}
