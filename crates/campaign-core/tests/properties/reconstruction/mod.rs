//! Deterministic reconstruction: replaying a history twice is equal, and every prefix of a valid
//! history reconstructs to the state recorded when that prefix was accepted.

fn prefixes_rebuild_recorded_states(
    history: &crate::suite::interpreter::History,
) -> Result<(), String> {
    let first = crate::suite::fixture::observed(campaign_core::reconstruct(&history.events))?;
    let second = crate::suite::fixture::observed(campaign_core::reconstruct(&history.events))?;
    assert_eq!(first, second);
    assert_eq!(&first, history.state()?);
    for (accepted, recorded) in history.checkpoints.iter().enumerate() {
        let prefix = history
            .events
            .get(..=accepted)
            .ok_or_else(|| String::from("every checkpoint has its prefix"))?;
        let replayed = crate::suite::fixture::observed(campaign_core::reconstruct(prefix))?;
        assert_eq!(&replayed, recorded);
        let mut recomputed = replayed.clone();
        crate::suite::fixture::observed(recomputed.recompute_identity())?;
        assert_eq!(recomputed, replayed);
        let control_events = u64::try_from(prefix.len()).map_err(|error| format!("{error}"))?;
        assert_eq!(recorded.control_event_count, control_events);
    }
    for planning in &history.plannings {
        assert!(
            history
                .checkpoints
                .iter()
                .any(|state| state == &planning.state)
        );
    }
    let last = history
        .events
        .last()
        .cloned()
        .ok_or_else(|| String::from("a history has its admission event"))?;
    let mut duplicated = history.events.clone();
    duplicated.push(last);
    assert_eq!(
        crate::suite::fixture::denied(campaign_core::reconstruct(&duplicated))?,
        campaign_core::CampaignError::DuplicateControlEvent
    );
    Ok(())
}

#[test]
// r[verify campaign.frontier.equal]
// r[verify campaign.functional_core.explicit]
fn every_history_prefix_reconstructs_its_recorded_state() -> Result<(), String> {
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &crate::suite::strategies::script(),
        |script| prefixes_rebuild_recorded_states(&crate::suite::interpreter::run(&script)?),
    )
}
