//! Round-12 shells kept as negative controls: each once passed every fixture because no fixture
//! presented the history it keys on through a held state. Load and cache shells that validate
//! cold once the presented lineage is two events ahead of a held state with one declared feature
//! value (finished, a retired, pruned, or no retained candidate, a discovery, a planner stop), or
//! once the branch generation advanced and the new events hold a deletion, a finding result, or a
//! revision, or the presented head is an exploratory selection; and a claims shell that claims
//! exhaustive search on a planner stop's earlier plan receipt once a consumer or policy stop is
//! recorded. Each now fails `history.state_coverage`, and only it, under the default and the
//! reordered profile, at a fork of the history before a held head or at the planner stop's plan
//! receipt.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

/// The state-coverage fixture.
const STATE_COVERAGE: &str = "history.state_coverage";

/// Failures of the whole suite under the default profile, which must equal those under the
/// reordered profile (`exploration_order = [1]`, `retention_rank_order = [1]`).
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let profile = super::default_profile()?;
    let mut reordered = profile.clone();
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    reordered.validate()?;
    let under = |profile: &crate::inputs::SuiteProfile| {
        let mut adapter = wrapper::ForkAdapter::new(holes);
        super::failures(&crate::catalog::run_suite(&mut adapter, profile))
    };
    let default = under(&profile);
    assert_eq!(under(&reordered), default, "reordered");
    Ok(default)
}

/// Whether a forbidden effect is a load or cache of a fork of the history before a held head.
fn is_fork(forbidden: &str) -> bool {
    forbidden.contains("a fork of the history before a held state's head")
}

/// Requires the state-coverage fixture to be the only failure, as a negative fixture that let a
/// forbidden effect `is_effect` accepts through.
fn only_state_coverage(
    holes: &[wrapper::Hole],
    is_effect: fn(&str) -> bool,
) -> crate::verdicts::ConformanceResult<()> {
    let failed = failures(holes)?;
    let names: alloc::vec::Vec<&str> = failed.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, [STATE_COVERAGE]);
    assert!(
        failed.iter().all(|(_, verdict)| matches!(
            verdict,
            crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden }
                if is_effect(forbidden)
        )),
        "{failed:?}"
    );
    Ok(())
}

/// Whether some candidate of `state` has status `is`.
fn any_candidate(
    state: &campaign_core::FrontierState,
    is: fn(&campaign_core::CandidateStatus) -> bool,
) -> bool {
    state
        .candidates
        .iter()
        .any(|candidate| is(&candidate.status))
}

/// Whether `event` deletes a candidate (a retirement or a prune).
fn is_deletion(event: &campaign_core::CampaignControlEvent) -> bool {
    matches!(
        event,
        campaign_core::CampaignControlEvent::CandidateRetired(_)
            | campaign_core::CampaignControlEvent::CandidatesPruned(_)
    )
}

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_fork_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
// r[verify campaign.stop.terminal]
fn a_resync_two_events_ahead_of_a_finished_holder_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        let state = presented.state();
        presented.is_ahead(2) && state.stopped.is_some() && state.outstanding.is_empty()
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_resync_two_events_ahead_of_a_holder_with_a_retired_candidate_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(2)
            && any_candidate(presented.state(), |status| {
                matches!(status, campaign_core::CandidateStatus::Retired { .. })
            })
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_resync_two_events_ahead_of_a_holder_with_a_pruned_candidate_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(2)
            && any_candidate(presented.state(), |status| {
                matches!(status, campaign_core::CandidateStatus::Pruned { .. })
            })
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_resync_two_events_ahead_of_a_holder_with_a_discovery_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(2) && !presented.state().discoveries.is_empty()
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_resync_two_events_ahead_of_a_planner_stopped_holder_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(2)
            && presented.state().stopped.as_ref().is_some_and(|stop| {
                !matches!(
                    stop.reason,
                    campaign_core::StopReason::ConsumerRequested { .. }
                        | campaign_core::StopReason::PolicyRequested { .. }
                )
            })
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_resync_two_events_ahead_of_a_holder_retaining_nothing_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(2)
            && !any_candidate(presented.state(), |status| {
                matches!(status, campaign_core::CandidateStatus::Retained)
            })
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_rebuild_on_an_advanced_deletion_head_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_advanced() && presented.head().is_some_and(is_deletion)
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_rebuild_on_an_advanced_suffix_deletion_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_advanced() && presented.new_events().any(is_deletion)
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_rebuild_on_an_advanced_finding_result_head_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_advanced()
            && matches!(
                presented.head(),
                Some(campaign_core::CampaignControlEvent::ResultObserved(result))
                    if !result.observation.findings.is_empty()
            )
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_rebuild_on_an_advanced_revision_head_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_advanced()
            && matches!(
                presented.head(),
                Some(campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance))
                    if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::Revision { .. })
            )
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_rebuild_on_a_forward_exploratory_selection_head_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let hole = wrapper::Hole::ColdWhen(|presented| {
        presented.is_ahead(0)
            && matches!(
                presented.head(),
                Some(campaign_core::CampaignControlEvent::SelectionPublished(selection))
                    if selection.entropy.is_some()
            )
    });
    only_state_coverage(&[hole], is_fork)
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.conformance.claims]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
fn a_stale_planner_stop_receipt_claim_after_a_requested_stop_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::StalePlannerStopClaim], |forbidden| {
        forbidden == "a planner stop's receipt planned before recording carried a promotion claim"
    })
}
