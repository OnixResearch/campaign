//! Deterministic frontier reconstruction (`campaign.frontier`).

const RANK_IMPROVED: i64 = 25;
const RANK_BEST: i64 = 30;
const SECOND_GENERATION: u64 = 2;

/// One canonical history: two guided candidates, one published and settled selection.
fn settled_history() -> Result<crate::suite::planning_support::History, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, alpha) = crate::suite::planning_support::guided(
        events,
        &state,
        b"alpha",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, _beta) = crate::suite::planning_support::guided(
        events,
        &state,
        b"beta",
        crate::suite::constants::RANK_LOW,
    )?;
    let (events, state, selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(selection.candidate_id, alpha);
    let settled = crate::suite::planning_support::settle(
        events,
        &state,
        &selection,
        (b"equal-child", b"equal-payload"),
        Vec::new(),
    )?;
    assert!(settled.1.outstanding.is_empty());
    Ok(settled)
}

#[test]
// r[verify campaign.frontier.equal]
// r[verify campaign.functional_core.explicit]
fn independently_built_equal_histories_reconstruct_equally() -> Result<(), String> {
    let (left_events, left) = settled_history()?;
    let (right_events, right) = settled_history()?;
    assert_eq!(left_events, right_events);
    assert_eq!(left, right);
    assert_eq!(left.state_id, right.state_id);
    let replayed =
        crate::suite::planning_support::observed(campaign_core::reconstruct(&right_events))?;
    assert_eq!(replayed.state_id, left.state_id);
    assert_eq!(
        left.selection_count,
        crate::suite::planning_support::FIRST_ORDINAL
    );
    Ok(())
}

#[test]
fn history_without_its_admission_is_a_typed_conflict() -> Result<(), String> {
    let (events, _state) = settled_history()?;
    assert_eq!(
        campaign_core::reconstruct(&[]),
        Err(campaign_core::CampaignError::MissingAdmission)
    );
    assert_eq!(
        campaign_core::reconstruct(&events[1..]),
        Err(campaign_core::CampaignError::MissingAdmission)
    );
    Ok(())
}

#[test]
fn a_repeated_or_second_admission_is_a_typed_conflict() -> Result<(), String> {
    let (events, _state) = settled_history()?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, events[0].clone()),
        Err(campaign_core::CampaignError::DuplicateControlEvent)
    );
    let mut other = crate::suite::admission::default_input()?;
    other.policy_id = crate::suite::admission::xid(campaign_core::IdKind::Policy, b"other");
    let (other_events, _other_state) = crate::suite::admission::admitted(other)?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, other_events[0].clone()),
        Err(campaign_core::CampaignError::InvalidControlEventOrder)
    );
    Ok(())
}

#[test]
// r[verify campaign.frontier.conflict]
fn duplicate_or_reordered_control_events_are_typed_conflicts() -> Result<(), String> {
    let (events, _state) = settled_history()?;
    let batch = events[1].clone();
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, batch),
        Err(campaign_core::CampaignError::DuplicateControlEvent)
    );
    let mut reordered = events.clone();
    reordered.swap(1, 2);
    assert_eq!(
        campaign_core::reconstruct(&reordered),
        Err(campaign_core::CampaignError::StaleState)
    );
    let mut skipped = events;
    skipped.remove(2);
    assert_eq!(
        campaign_core::reconstruct(&skipped),
        Err(campaign_core::CampaignError::StaleState)
    );
    Ok(())
}

#[test]
// r[verify campaign.frontier.conflict]
fn a_crossed_selection_is_a_typed_conflict() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, alpha) = crate::suite::planning_support::guided(
        events,
        &state,
        b"alpha",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, beta) = crate::suite::planning_support::guided(
        events,
        &state,
        b"beta",
        crate::suite::constants::RANK_LOW,
    )?;
    let selection =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    assert_eq!(selection.candidate_id, alpha);
    let beta_guidance = crate::suite::planning_support::observed(state.candidate(beta))?
        .latest_guidance
        .clone()
        .ok_or_else(|| String::from("beta must be guided"))?;
    let crossed_guidance =
        crate::suite::planning_support::forged_selection(&selection, |decision| {
            decision.guidance_id = beta_guidance.guidance_id;
        })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, crossed_guidance),
        Err(campaign_core::CampaignError::StaleGuidance)
    );
    let crossed_candidate =
        crate::suite::planning_support::forged_selection(&selection, |decision| {
            decision.candidate_id = beta;
        })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, crossed_candidate),
        Err(campaign_core::CampaignError::StaleGuidance)
    );
    let crossed_parent =
        crate::suite::planning_support::forged_selection(&selection, |decision| {
            decision.parent_moment_id =
                crate::suite::admission::xid(campaign_core::IdKind::RootMoment, b"other-root");
        })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, crossed_parent),
        Err(campaign_core::CampaignError::CrossedIdentity)
    );
    let replayed = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(replayed.state_id, state.state_id);
    Ok(())
}

#[test]
// r[verify campaign.frontier.conflict]
fn conflicting_successors_are_never_resolved_by_sort_order() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::planning_support::guided(
        events,
        &state,
        b"contested",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (improved_events, _improved, _first) = crate::suite::planning_support::guide_ranks(
        events.clone(),
        &state,
        candidate_id,
        &[RANK_IMPROVED],
        SECOND_GENERATION,
    )?;
    let (best_events, _best, _second) = crate::suite::planning_support::guide_ranks(
        events.clone(),
        &state,
        candidate_id,
        &[RANK_BEST],
        SECOND_GENERATION,
    )?;
    let improved = improved_events
        .last()
        .cloned()
        .ok_or_else(|| String::from("improved guidance event must exist"))?;
    let best = best_events
        .last()
        .cloned()
        .ok_or_else(|| String::from("best guidance event must exist"))?;
    assert_ne!(improved.event_id(), best.event_id());
    assert_eq!(improved.state_before_id(), best.state_before_id());
    for (first, second) in [(&improved, &best), (&best, &improved)] {
        let mut conflicting = events.clone();
        conflicting.push(first.clone());
        conflicting.push(second.clone());
        assert_eq!(
            campaign_core::reconstruct(&conflicting),
            Err(campaign_core::CampaignError::StaleState)
        );
    }
    Ok(())
}

#[test]
// r[verify campaign.frontier.selected]
fn a_selected_candidate_counts_its_selections_and_stays_selectable() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::planning_support::guided(
        events,
        &state,
        b"reused",
        crate::suite::constants::RANK_HIGH,
    )?;
    assert_eq!(
        crate::suite::planning_support::observed(state.candidate(candidate_id))?.selection_count,
        0
    );
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let replayed = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(replayed, state);
    assert_eq!(
        crate::suite::planning_support::observed(state.candidate(candidate_id))?.selection_count,
        crate::suite::planning_support::FIRST_ORDINAL
    );
    assert!(matches!(
        crate::suite::planning_support::observed(state.candidate(candidate_id))?.status,
        campaign_core::CandidateStatus::Retained
    ));
    let while_outstanding =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    assert_eq!(while_outstanding.candidate_id, candidate_id);
    let (events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &first,
        (b"reused-child", b"reused-payload"),
        Vec::new(),
    )?;
    let (_events, state, second) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(second.candidate_id, candidate_id);
    assert_eq!(
        second.ordinal,
        crate::suite::planning_support::SECOND_ORDINAL
    );
    assert_eq!(
        crate::suite::planning_support::observed(state.candidate(candidate_id))?.selection_count,
        crate::suite::planning_support::SECOND_ORDINAL
    );
    Ok(())
}
