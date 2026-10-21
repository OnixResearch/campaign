//! Traversal bounds: every collection and counter of a reconstructed state stays within its
//! declared bound, and generated histories reach those bounds as typed denials.

fn length(items: usize) -> Result<u64, String> {
    u64::try_from(items).map_err(|error| format!("{error}"))
}

fn state_is_bounded(state: &campaign_core::FrontierState) -> Result<(), String> {
    let bounds = &state.admission.bounds;
    assert!(length(state.candidates.len())? <= bounds.max_candidates);
    assert!(length(state.outstanding.len())? <= bounds.max_outstanding);
    assert!(state.selection_count <= bounds.max_selections);
    assert!(state.guidance_record_count <= bounds.max_guidance_records);
    assert!(state.control_event_count <= bounds.max_control_events);
    assert_eq!(length(state.event_ids.len())?, state.control_event_count);
    assert!(length(state.findings.len())? <= bounds.max_findings);
    assert!(state.reserved_resources.len() <= state.admission.resource_budgets.len());
    let settled_or_outstanding = state
        .settled
        .len()
        .checked_add(state.outstanding.len())
        .ok_or_else(|| String::from("selection counts overflow"))?;
    assert_eq!(length(settled_or_outstanding)?, state.selection_count);
    assert!(state.accepted_results.len() <= state.settled.len());
    let child_moments = state
        .known_moments
        .len()
        .checked_sub(state.admission.roots.len())
        .ok_or_else(|| String::from("roots stay known"))?;
    let child_bound = bounds
        .max_child_moments
        .checked_mul(length(state.accepted_results.len())?)
        .ok_or_else(|| String::from("child bound overflow"))?;
    assert!(length(child_moments)? <= child_bound);
    let mut selected = 0_u64;
    for candidate in &state.candidates {
        assert!(candidate.candidate.depth <= bounds.max_depth);
        assert!(state.has_known_moment(candidate.candidate.parent_moment_id));
        if let Some(guidance) = &candidate.latest_guidance {
            assert!(length(guidance.ranks.len())? <= bounds.max_rank_dimensions);
            assert!(
                guidance
                    .ranks
                    .iter()
                    .all(|rank| rank.unsigned_abs() <= bounds.max_rank_magnitude)
            );
        }
        selected = selected
            .checked_add(candidate.selection_count)
            .ok_or_else(|| String::from("selection counts overflow"))?;
    }
    assert_eq!(selected, state.selection_count);
    Ok(())
}

#[test]
// r[verify campaign.budgets]
fn reconstructed_states_stay_within_declared_limits() -> Result<(), String> {
    let denials = core::cell::Cell::new(0_u64);
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &crate::suite::strategies::script(),
        |script| {
            let history = crate::suite::interpreter::run(&script)?;
            for state in &history.checkpoints {
                state_is_bounded(state)?;
            }
            let reached = history.rejections.iter().filter(|rejection| {
                matches!(
                    rejection.error,
                    campaign_core::CampaignError::BoundExceeded
                        | campaign_core::CampaignError::ControlEventLimitReached
                )
            });
            for rejection in reached.clone() {
                state_is_bounded(&rejection.state)?;
            }
            let reached_count = length(reached.count())?;
            assert!(reached_count <= length(history.rejections.len())?);
            denials.set(denials.get().saturating_add(reached_count));
            Ok(())
        },
    )?;
    assert!(
        denials.get() > 0,
        "no generated history reached a declared bound"
    );
    Ok(())
}
