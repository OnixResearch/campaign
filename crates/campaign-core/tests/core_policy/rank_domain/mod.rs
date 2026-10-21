//! `campaign.configuration.invalid` decay domain at planning and replay: at the exact edge
//! `penalty * (max_selections - 1) + max_rank_magnitude == 2^63`, every plannable selection plans
//! and replays, and one more unit of penalty is `InvalidConfig` before any selection.

/// Rank magnitude and selection bound of the edge; the penalty closes `2^63` exactly.
const MAGNITUDE: u64 = 1 << 62;
const SELECTIONS: u64 = 3;
const EDGE_PENALTY: u64 = 1 << 61;

type Events = Vec<campaign_core::CampaignControlEvent>;

fn decayed(penalty: u64) -> Result<campaign_core::SelectionProfile, String> {
    let mut profile = crate::suite::selection::ranked_profile()?;
    profile.profile_id = campaign_core::Id::absent();
    profile.decay = campaign_core::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection: penalty,
    };
    profile.derive().map_err(crate::suite::s3_fixture::err)
}

fn plan(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
) -> campaign_core::CampaignResult<campaign_core::DecisionPlan> {
    campaign_core::plan_next(
        state,
        crate::suite::selection::request(state, profile.clone(), None),
        &state.admission.projection,
    )
}

/// Plans one selection of `candidate` under `profile`, publishes it, and settles it.
fn select_and_settle(
    events: Events,
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    candidate: campaign_core::Id,
) -> Result<(Events, campaign_core::FrontierState), String> {
    let planned = plan(state, profile).map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::NextDecision::Select(selection) = planned.decision else {
        return Err(format!("expected a selection, got {:?}", planned.decision));
    };
    assert_eq!(selection.candidate_id, candidate);
    let event = crate::suite::selection::event_with(*selection.clone(), profile.clone(), None)?;
    let (events, selected) = crate::suite::s3_fixture::append(events, event)?;
    let result = campaign_core::admit_result(
        &selected,
        crate::suite::s3_fixture::observation(&selected, &selection, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, settled) = crate::suite::s3_fixture::append(events, result)?;
    assert!(settled.outstanding.is_empty());
    Ok((events, settled))
}

/// A candidate at the lowest admissible rank is decayed to `i64::MIN` on the last plannable
/// selection, which plans, replays, and then stops at the selection limit; one more unit of
/// penalty is `InvalidConfig` up front.
#[test]
fn the_exact_decay_edge_reaches_i64_min_and_one_over_is_invalid_config() -> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_selections = SELECTIONS;
        bounds.max_rank_magnitude = MAGNITUDE;
    })?;
    let lowest = -i64::try_from(MAGNITUDE).map_err(crate::suite::s3_fixture::err)?;
    let (mut events, mut state, candidate) =
        crate::suite::s3_fixture::guided(events, &state, b"decay-edge-lowest", lowest)?;
    let edge = decayed(EDGE_PENALTY)?;
    let over = decayed(EDGE_PENALTY.checked_add(1).ok_or("penalty overflow")?)?;
    assert_eq!(
        plan(&state, &over).map(|_| ()),
        Err(campaign_core::CampaignError::InvalidConfig)
    );
    for _ in 0..SELECTIONS {
        (events, state) = select_and_settle(events, &state, &edge, candidate)?;
    }
    assert_eq!(state.selection_count, SELECTIONS);
    let last = plan(&state, &edge).map_err(crate::suite::s3_fixture::err)?;
    assert!(matches!(
        last.decision,
        campaign_core::NextDecision::Stop(ref stop)
            if stop.reason == campaign_core::StopReason::SelectionLimitReached
    ));
    assert_eq!(
        campaign_core::reconstruct(&events).map_err(crate::suite::s3_fixture::err)?,
        state
    );
    Ok(())
}
