const THRESHOLD: u64 = 500_000;
const EXPLOIT_DRAW: u64 = 900_000;
const EXPLORE_DRAW: u64 = 100;
const INDEX_DRAW: u64 = 4;
const SECOND_RANK_DIMENSION: u32 = 1;
const OUT_OF_RANGE_DIMENSION: u32 = 16;
const ALPHA_RANKS: [i64; 2] = [20, 1];
const BETA_RANKS: [i64; 2] = [15, 5];
const GAMMA_RANKS: [i64; 2] = [10, 3];
const CANDIDATE_COUNT: u64 = 3;
const DRAW_BYTES: usize = 8;
const INDEX_DRAW_END: usize = 16;

struct Frontier {
    state: campaign_core::FrontierState,
    alpha: campaign_core::Id,
    gamma: campaign_core::Id,
}

fn guide_ranks(
    mut events: Vec<campaign_core::CampaignControlEvent>,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    ranks: &[i64],
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    let event = campaign_core::admit_guidance(
        state,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            candidate_id,
            generation: crate::suite::constants::GENERATION_NEXT,
            ranks: ranks.to_vec(),
            source_facts_id: crate::suite::admission::xid(
                campaign_core::IdKind::SourceFacts,
                b"exploration-facts",
            ),
            ordering_profile_id: state.admission.guidance_profile_id,
            adapter_id: state.admission.adapter_id,
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    events.push(event);
    let next = campaign_core::reconstruct(&events).map_err(|error| format!("{error:?}"))?;
    let latest = next
        .candidate(candidate_id)
        .map_err(|error| format!("{error:?}"))?
        .latest_guidance
        .as_ref()
        .map(|guidance| guidance.ranks.clone());
    assert_eq!(latest, Some(ranks.to_vec()));
    assert_ne!(next.state_id, state.state_id);
    Ok((events, next))
}

fn frontier() -> Result<Frontier, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, alpha) = crate::suite::candidate::admit(events, &state, b"alpha", cost)?;
    let (events, state, beta) = crate::suite::candidate::admit(events, &state, b"beta", cost)?;
    let (events, state, gamma) = crate::suite::candidate::admit(events, &state, b"gamma", cost)?;
    let (events, state) = guide_ranks(events, &state, alpha, &ALPHA_RANKS)?;
    let (events, state) = guide_ranks(events, &state, beta, &BETA_RANKS)?;
    let (_events, state) = guide_ranks(events, &state, gamma, &GAMMA_RANKS)?;
    Ok(Frontier {
        state,
        alpha,
        gamma,
    })
}

fn exploratory_profile(order: Vec<u32>) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::selection::profile(campaign_core::SelectionMode::Exploratory {
        threshold_parts_per_million: THRESHOLD,
        entropy_source_id: source(),
        exploration_order: order,
    })
}

fn source() -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::EntropyProfile, b"entropy-profile")
}

fn ticket(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    explore_draw: u64,
) -> Result<campaign_core::EntropyTicket, String> {
    let mut bytes = [0; campaign_core::DIGEST_LENGTH];
    bytes[..DRAW_BYTES].copy_from_slice(&explore_draw.to_le_bytes());
    bytes[DRAW_BYTES..INDEX_DRAW_END].copy_from_slice(&INDEX_DRAW.to_le_bytes());
    let ticket = campaign_core::EntropyTicket {
        entropy_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile_id: profile.profile_id,
        source_id: source(),
        counter: crate::suite::constants::ENTROPY_COUNTER,
        bytes,
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(ticket.explore_draw(), explore_draw);
    assert_eq!(ticket.index_draw(), INDEX_DRAW);
    Ok(ticket)
}

fn plan_with_draw(
    frontier: &Frontier,
    explore_draw: u64,
) -> Result<campaign_core::DecisionPlan, String> {
    let profile = exploratory_profile(vec![SECOND_RANK_DIMENSION])?;
    let entropy = ticket(&frontier.state, &profile, explore_draw)?;
    campaign_core::plan_next(
        &frontier.state,
        crate::suite::selection::request(&frontier.state, profile, Some(entropy)),
        &frontier.state.admission.projection,
    )
    .map_err(|error| format!("{error:?}"))
}

fn selected(plan: &campaign_core::DecisionPlan) -> Result<campaign_core::Id, String> {
    let campaign_core::NextDecision::Select(selection) = &plan.decision else {
        return Err(format!("expected a selection, got {:?}", plan.decision));
    };
    Ok(selection.candidate_id)
}

#[test]
fn exploit_branch_selects_the_first_ranked_candidate() -> Result<(), String> {
    let frontier = frontier()?;
    let plan = plan_with_draw(&frontier, EXPLOIT_DRAW)?;
    assert_eq!(selected(&plan)?, frontier.alpha);
    assert_eq!(
        plan.work,
        campaign_core::PlanningWork {
            candidates_examined: CANDIDATE_COUNT,
            eligible_candidates: CANDIDATE_COUNT,
        }
    );
    Ok(())
}

#[test]
// r[verify campaign.entropy.valid]
fn explore_branch_indexes_the_declared_rank_order() -> Result<(), String> {
    // Exploration order over rank dimension 1 is beta(5), gamma(3), alpha(1);
    // index_draw 4 % 3 = 1 selects gamma, which is neither ranked first nor ranked second.
    let frontier = frontier()?;
    let plan = plan_with_draw(&frontier, EXPLORE_DRAW)?;
    assert_eq!(selected(&plan)?, frontier.gamma);
    assert_eq!(plan, plan_with_draw(&frontier, EXPLORE_DRAW)?);
    Ok(())
}

#[test]
fn declared_order_outside_the_rank_bound_is_rejected() -> Result<(), String> {
    let frontier = frontier()?;
    let profile = exploratory_profile(vec![OUT_OF_RANGE_DIMENSION])?;
    let entropy = ticket(&frontier.state, &profile, EXPLORE_DRAW)?;
    assert_eq!(
        campaign_core::plan_next(
            &frontier.state,
            crate::suite::selection::request(&frontier.state, profile, Some(entropy)),
            &frontier.state.admission.projection,
        ),
        Err(campaign_core::CampaignError::InvalidConfig)
    );
    Ok(())
}
