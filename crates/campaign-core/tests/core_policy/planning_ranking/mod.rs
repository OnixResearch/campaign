//! Deterministic ranking and checked selection-count decay (`campaign.guidance.valid`,
//! `campaign.selection.decay`).

const RANK_TOP: i64 = 20;
const RANK_NEAR: i64 = 18;
const RANK_MIDDLE: i64 = 11;
const RANK_BOTTOM: i64 = 10;
const RANK_SHARED: i64 = 5;
const RANK_SECOND_LOW: i64 = 1;
const RANK_SECOND_HIGH: i64 = 3;
const RANK_FIRST_LOWER: i64 = 4;
const RANK_SECOND_MAX: i64 = 9;
const PENALTY_OVERTAKE: u64 = 5;
const PENALTY_TIE: u64 = 2;

fn decay_profile(
    decay: campaign_core::SelectionCountDecay,
) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::planning_support::profile_with(|profile| profile.decay = decay)
}

fn chosen(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
) -> Result<campaign_core::Id, String> {
    let plan = crate::suite::planning_support::observed(
        crate::suite::planning_support::plan_with(state, profile, None),
    )?;
    Ok(crate::suite::planning_support::selected(&plan)?.candidate_id)
}

fn two_dimensional(
    label: &'static [u8],
    ranks: &[i64],
    history: crate::suite::planning_support::History,
) -> Result<(crate::suite::planning_support::History, campaign_core::Id), String> {
    let (events, state) = history;
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, &state, label, crate::suite::constants::COST_ONE)?;
    let (events, state, _guidance) = crate::suite::planning_support::guide_ranks(
        events,
        &state,
        candidate_id,
        ranks,
        crate::suite::planning_support::FIRST_GENERATION,
    )?;
    Ok(((events, state), candidate_id))
}

#[test]
// r[verify campaign.selection.valid]
fn the_highest_first_rank_is_selected() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, _low) =
        crate::suite::planning_support::guided(events, &state, b"low", RANK_BOTTOM)?;
    let (events, state, high) =
        crate::suite::planning_support::guided(events, &state, b"high", RANK_TOP)?;
    let (_events, state, _middle) =
        crate::suite::planning_support::guided(events, &state, b"middle", RANK_MIDDLE)?;
    let plan = crate::suite::planning_support::ranked(&state)?;
    assert_eq!(
        crate::suite::planning_support::selected(&plan)?.candidate_id,
        high
    );
    assert_eq!(plan, crate::suite::planning_support::ranked(&state)?);
    Ok(())
}

#[test]
// r[verify campaign.guidance.valid]
fn later_declared_dimensions_only_break_earlier_ties() -> Result<(), String> {
    let history = crate::suite::admission::initial_state()?;
    let (history, _lower_second) =
        two_dimensional(b"lower-second", &[RANK_SHARED, RANK_SECOND_LOW], history)?;
    let (history, higher_second) =
        two_dimensional(b"higher-second", &[RANK_SHARED, RANK_SECOND_HIGH], history)?;
    let ((_events, state), lower_first) = two_dimensional(
        b"lower-first",
        &[RANK_FIRST_LOWER, RANK_SECOND_MAX],
        history,
    )?;
    assert_eq!(
        chosen(
            &state,
            decay_profile(campaign_core::SelectionCountDecay::None)?
        )?,
        higher_second
    );
    assert_ne!(higher_second, lower_first);
    Ok(())
}

#[test]
// r[verify campaign.guidance.valid]
fn equal_ranks_fall_back_to_ascending_candidate_identity() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, left) =
        crate::suite::planning_support::guided(events, &state, b"left", RANK_TOP)?;
    let (_events, state, right) =
        crate::suite::planning_support::guided(events, &state, b"right", RANK_TOP)?;
    assert_ne!(left, right);
    assert_eq!(
        chosen(
            &state,
            decay_profile(campaign_core::SelectionCountDecay::None)?
        )?,
        left.min(right)
    );
    Ok(())
}

#[test]
// r[verify campaign.selection.decay]
fn selection_count_decay_lowers_the_effective_first_rank() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, top) =
        crate::suite::planning_support::guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, near) =
        crate::suite::planning_support::guided(events, &state, b"near", RANK_NEAR)?;
    let (_events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(first.candidate_id, top);
    let overtake = decay_profile(campaign_core::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection: PENALTY_OVERTAKE,
    })?;
    assert_eq!(chosen(&state, overtake)?, near);
    assert_eq!(
        chosen(
            &state,
            decay_profile(campaign_core::SelectionCountDecay::None)?
        )?,
        top
    );
    Ok(())
}

#[test]
// r[verify campaign.selection.decay]
fn a_tie_after_decay_uses_ascending_candidate_identity() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, one) =
        crate::suite::candidate::admit(events, &state, b"one", crate::suite::constants::COST_ONE)?;
    let (events, state, two) =
        crate::suite::candidate::admit(events, &state, b"two", crate::suite::constants::COST_ONE)?;
    let (lower_id, higher_id) = (one.min(two), one.max(two));
    let (events, state, _g) = crate::suite::planning_support::guide_ranks(
        events,
        &state,
        higher_id,
        &[RANK_TOP],
        crate::suite::planning_support::FIRST_GENERATION,
    )?;
    let (events, state, _g) = crate::suite::planning_support::guide_ranks(
        events,
        &state,
        lower_id,
        &[RANK_NEAR],
        crate::suite::planning_support::FIRST_GENERATION,
    )?;
    let (_events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(first.candidate_id, higher_id);
    let tie = decay_profile(campaign_core::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection: PENALTY_TIE,
    })?;
    assert_eq!(chosen(&state, tie)?, lower_id);
    Ok(())
}

/// Largest `subtract_from_first_rank` penalty representable under `bounds`:
/// `floor((2^63 - max_rank_magnitude) / (max_selections - 1))`.
fn largest_representable_penalty(bounds: &campaign_core::ExplorationBounds) -> Result<u64, String> {
    let prior_selections = bounds
        .max_selections
        .checked_sub(1)
        .ok_or("the fixture bounds allow a prior selection")?;
    i64::MIN
        .unsigned_abs()
        .checked_sub(bounds.max_rank_magnitude)
        .and_then(|headroom| headroom.checked_div(prior_selections))
        .ok_or_else(|| String::from("the fixture decay headroom is representable"))
}

#[test]
// r[verify campaign.selection.decay]
fn a_decay_unrepresentable_under_the_admission_bounds_is_invalid_config() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, top) =
        crate::suite::planning_support::guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, bottom) =
        crate::suite::planning_support::guided(events, &state, b"bottom", RANK_BOTTOM)?;
    let edge = largest_representable_penalty(&state.admission.bounds)?;
    let decay = |penalty_per_selection| {
        decay_profile(campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection,
        })
    };
    let over_edge = edge
        .checked_add(1)
        .ok_or("the fixture edge penalty is below u64::MAX")?;
    // Rejected before any selection, although no candidate has been decayed yet.
    for penalty in [over_edge, u64::MAX] {
        assert_eq!(
            crate::suite::planning_support::plan_with(&state, decay(penalty)?, None),
            Err(campaign_core::CampaignError::InvalidConfig),
            "penalty {penalty}"
        );
    }
    assert_eq!(chosen(&state, decay(edge)?)?, top);
    let (_events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(first.candidate_id, top);
    assert_eq!(
        crate::suite::planning_support::plan_with(&state, decay(u64::MAX)?, None),
        Err(campaign_core::CampaignError::InvalidConfig)
    );
    // At the edge the decayed top rank drops below the undecayed bottom rank.
    assert_eq!(chosen(&state, decay(edge)?)?, bottom);
    Ok(())
}
