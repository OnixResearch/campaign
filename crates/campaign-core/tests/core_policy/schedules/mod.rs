//! Shared frontier and profiles for `campaign.fd.scheduling` controls.

const RANK_FIRST: i64 = 20;
const RANK_SECOND: i64 = 15;
pub(crate) const HEAVY_PENALTY: u64 = 10;
pub(crate) const FINDING_LIMIT: u64 = 1;
const INDEX_DRAW_START: usize = 8;
const INDEX_DRAW_END: usize = 16;
pub(crate) const EXPLORE_FIRST: u64 = 0;
pub(crate) const EXPLORE_SECOND: u64 = 1;

pub(crate) struct Frontier {
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) first: campaign_core::Id,
    pub(crate) second: campaign_core::Id,
}

/// Two guided candidates; the higher-ranked one was selected once and settled.
pub(crate) fn frontier() -> Result<Frontier, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, first) = crate::suite::candidate::admit(events, &state, b"first", cost)?;
    let (events, state, second) = crate::suite::candidate::admit(events, &state, b"second", cost)?;
    let (events, state, _g) =
        crate::suite::candidate::guide(events, &state, first, RANK_FIRST, cost)?;
    let (events, state, _g) =
        crate::suite::candidate::guide(events, &state, second, RANK_SECOND, cost)?;
    let selection = crate::suite::selection::plan(&state)?;
    assert_eq!(selection.candidate_id, first);
    let (mut events, state) = crate::suite::selection::publish(events, selection.clone())?;
    events.push(crate::suite::world::observed(campaign_core::admit_result(
        &state,
        crate::suite::observation::result_for(
            &state,
            &selection,
            b"sched-child",
            b"sched-payload",
        )?,
    ))?);
    let state = crate::suite::world::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(state.accepted_results.len(), 1);
    Ok(Frontier {
        state,
        first,
        second,
    })
}

pub(crate) fn variant(
    change: impl FnOnce(&mut campaign_core::SelectionProfile),
) -> Result<campaign_core::SelectionProfile, String> {
    let mut profile = crate::suite::selection::ranked_profile()?;
    change(&mut profile);
    crate::suite::world::observed(profile.derive())
}

pub(crate) fn undecayed() -> Result<campaign_core::SelectionProfile, String> {
    variant(|profile| profile.decay = campaign_core::SelectionCountDecay::None)
}

pub(crate) fn planned(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> Result<(campaign_core::DecisionPlan, campaign_core::SelectionRequest), String> {
    crate::suite::schedules::planned_on(state, profile, entropy, &state.admission.projection)
}

/// Plans `profile` and `entropy` on `state` against `binding`.
pub(crate) fn planned_on(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
    binding: &campaign_core::ProjectionBinding,
) -> Result<(campaign_core::DecisionPlan, campaign_core::SelectionRequest), String> {
    let request = crate::suite::selection::request_on(state, profile, entropy, binding);
    let plan =
        crate::suite::world::observed(campaign_core::plan_next(state, request.clone(), binding))?;
    Ok((plan, request))
}

pub(crate) fn selected(plan: &campaign_core::DecisionPlan) -> Result<campaign_core::Id, String> {
    match &plan.decision {
        campaign_core::NextDecision::Select(selection) => Ok(selection.candidate_id),
        other => Err(format!("expected a selection, got {other:?}")),
    }
}

pub(crate) fn forge_selected(
    mut plan: campaign_core::DecisionPlan,
    candidate_id: campaign_core::Id,
) -> campaign_core::DecisionPlan {
    if let campaign_core::NextDecision::Select(selection) = &mut plan.decision {
        selection.candidate_id = candidate_id;
    }
    plan
}

pub(crate) fn ticket(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    index_draw: u64,
) -> Result<campaign_core::EntropyTicket, String> {
    let mut bytes = [0_u8; campaign_core::DIGEST_LENGTH];
    bytes[INDEX_DRAW_START..INDEX_DRAW_END].copy_from_slice(&index_draw.to_le_bytes());
    crate::suite::world::observed(
        campaign_core::EntropyTicket {
            entropy_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            profile_id: profile.profile_id,
            source_id: state.admission.entropy_profile_id,
            counter: crate::suite::constants::ENTROPY_COUNTER,
            bytes,
        }
        .derive(),
    )
}

pub(crate) fn always_explore(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionProfile, String> {
    let entropy_source_id = state.admission.entropy_profile_id;
    variant(|profile| {
        profile.decay = campaign_core::SelectionCountDecay::None;
        profile.mode = campaign_core::SelectionMode::Exploratory {
            threshold_parts_per_million: crate::suite::constants::ENTROPY_ALWAYS,
            entropy_source_id,
            exploration_order: vec![crate::suite::constants::FIRST_RANK_DIMENSION],
        };
    })
}

pub(crate) fn declared_of(
    profile: &campaign_core::SelectionProfile,
) -> Result<campaign_core::SchedulingRules, String> {
    crate::suite::world::observed(campaign_core::SchedulingRules::from_profile(profile))
}
