//! Round-11 state-keyed shells kept as negative controls: each shell once passed every fixture
//! because no fixture paired the state fact it keys on with a presentation. A load and cache
//! shell that validates cold whenever its held state is finished, has a retired or pruned
//! candidate, or has a discovery; a claims shell that calls a finding-free exhausted or stopped
//! campaign searched or releasable; and a claims shell that claims safe deletion on the event
//! receipt of one event kind against a state of one stop reason now fail `history.state_coverage`
//! under the default and the reordered profile, as does a shell keyed on a 4-way conjunction of
//! declared feature values the covering table carries.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

/// The state-coverage fixture.
const STATE_COVERAGE: &str = "history.state_coverage";

/// The default profile and the reordered profile (`exploration_order = [1]`,
/// `retention_rank_order = [1]`).
fn profiles() -> crate::verdicts::ConformanceResult<[crate::inputs::SuiteProfile; 2]> {
    let profile = super::default_profile()?;
    let mut reordered = profile.clone();
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    reordered.validate()?;
    Ok([profile, reordered])
}

/// Failures of the whole suite under the default profile, which must equal those under the
/// reordered profile.
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let [profile, reordered] = profiles()?;
    let under = |profile: &crate::inputs::SuiteProfile| {
        let mut adapter = wrapper::StateAdapter::new(holes);
        super::failures(&crate::catalog::run_suite(&mut adapter, profile))
    };
    let default = under(&profile);
    assert_eq!(under(&reordered), default, "reordered");
    Ok(default)
}

/// Names of the failed fixtures, in suite order.
fn names(failures: &Failures) -> alloc::vec::Vec<&'static str> {
    failures.iter().map(|(name, _)| *name).collect()
}

/// Requires the state-coverage fixture to be the only failure, and to fail as a negative fixture
/// that let a forbidden effect through.
fn only_state_coverage(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<()> {
    let failed = failures(holes)?;
    assert_eq!(names(&failed), [STATE_COVERAGE]);
    assert!(failed.iter().all(|(_, verdict)| matches!(
        verdict,
        crate::verdicts::FixtureVerdict::NegativeFixturePassed { .. }
    )));
    Ok(())
}

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_state_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_cold_reload_once_the_held_campaign_is_finished_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ColdWhen(wrapper::Held::Finished)])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
fn a_cold_reload_once_a_candidate_is_retired_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ColdWhen(wrapper::Held::Retired)])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
fn a_cold_reload_once_a_candidate_is_pruned_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ColdWhen(wrapper::Held::Pruned)])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
fn a_cold_reload_once_a_candidate_is_retired_or_pruned_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ColdWhen(wrapper::Held::RetiredOrPruned)])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
fn a_cold_reload_once_a_child_is_discovered_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ColdWhen(wrapper::Held::Discoveries)])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.claims]
fn a_finding_free_exhaustion_overclaim_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ClaimCleanExhaustion])
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.claims]
fn a_finding_free_stop_release_overclaim_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    only_state_coverage(&[wrapper::Hole::ClaimCleanStopRelease])
}

mod pairs;
