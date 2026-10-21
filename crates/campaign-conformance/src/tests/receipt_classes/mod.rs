//! Tenth closeout-audit claim loophole adapters kept as negative controls: each shell that once
//! passed every fixture because the claims fixtures presented event receipts only against the
//! final consumer-stopped state (never against a running state, a planner-stopped state, or the
//! adapter's host-capacity stop, which no fixture recorded), no result receipt reporting findings
//! or children, and no plan receipt of a searched frontier's stop before it was recorded, now
//! fails exactly the named fixtures that present the missing class, with their declared
//! verdicts, under the default and the reordered profile, in debug and release builds.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::TenthAuditAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

/// Failures under the default profile, which must equal those under the reordered profile
/// (`exploration_order = [1]`, `retention_rank_order = [1]`).
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let profile = super::default_profile()?;
    let mut reordered = profile.clone();
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    reordered.validate()?;
    let default = failures_under(holes, &profile);
    assert_eq!(failures_under(holes, &reordered), default, "reordered");
    Ok(default)
}

/// Negative fixtures that let through the named forbidden effects.
fn passed(declared: &[(&'static str, &'static str)]) -> Failures {
    declared
        .iter()
        .map(|(name, forbidden)| {
            (
                *name,
                crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
            )
        })
        .collect()
}

const NON_SELECTION: &str = "receipts.non_selection_overclaim";
const PLANNER: &str = "receipts.planner_overclaim";
const RECORDED: &str = "receipts.recorded_stop_overclaim";
const ADMISSION_CLAIM: &str = "a campaign admission event receipt carried a promotion claim";
const RESULT_CLAIM: &str = "a result event receipt carried a promotion claim";

/// The fixture that catches a stopped-state event claim for each stop class: the recorded-stop
/// fixture for every planner stop and the host-capacity stop, the non-selection fixture for the
/// consumer stop.
const STOPPED_EVENT_FIXTURES: [(wrapper::StopClass, &str); 7] = [
    (wrapper::StopClass::Frontier, RECORDED),
    (wrapper::StopClass::Budget, RECORDED),
    (wrapper::StopClass::SelectionLimit, RECORDED),
    (wrapper::StopClass::ControlEventLimit, RECORDED),
    (wrapper::StopClass::Policy, RECORDED),
    (wrapper::StopClass::HostCapacity, RECORDED),
    (wrapper::StopClass::Consumer, NON_SELECTION),
];

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A consumer that re-issues its result receipts as "search complete" once the search stopped
/// claims exhaustive search on a result's event receipt projected against the stopped state of a
/// searched frontier.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
// r[verify campaign.conformance.claims]
fn an_exhausted_result_claim_fails_the_recorded_stop_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ExhaustedResult])?,
        crate::tests::state_coverage_let_through(
            passed(&[(RECORDED, RESULT_CLAIM)]),
            "a result event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// A consumer that claims exhaustive search on any event receipt but the stop's own, projected
/// against a state stopped for one reason, fails on the admission's receipt against that state:
/// for every planner stop reason and the host-capacity stop in the recorded-stop fixture, for the
/// consumer stop in the non-selection fixture.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
// r[verify campaign.conformance.claims]
fn stopped_state_event_claims_fail_for_every_stop_reason() -> crate::verdicts::ConformanceResult<()>
{
    for (class, fixture) in STOPPED_EVENT_FIXTURES {
        assert_eq!(
            failures(&[wrapper::Hole::StoppedEvent(class)])?,
            crate::tests::state_coverage_let_through(
                passed(&[(fixture, ADMISSION_CLAIM)]),
                "a campaign admission event receipt carried a promotion claim"
            ),
            "{class:?}"
        );
    }
    Ok(())
}

/// A consumer that claims exhaustive search on every receipt naming the adapter's host-capacity
/// stop or a state it stopped fails once the recorded-stop fixture records that stop through a
/// tight host graph bound.
#[test]
// r[verify campaign.stop.host_capacity]
// r[verify campaign.receipts.host_capacity]
// r[verify campaign.conformance.claims]
fn a_host_capacity_stop_claim_fails_the_recorded_stop_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CapacityStop])?,
        crate::tests::state_coverage_let_through(
            passed(&[(RECORDED, ADMISSION_CLAIM)]),
            "a campaign admission event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Claims keyed on a result's reported findings (finding correctness) or children (event truth)
/// fail every claims fixture that settles such a result.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.negative]
fn finding_and_child_result_claims_fail_the_claims_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::FindingResult])?,
        crate::tests::state_coverage_let_through(
            passed(&[
                (NON_SELECTION, RESULT_CLAIM),
                (PLANNER, RESULT_CLAIM),
                (RECORDED, RESULT_CLAIM),
            ]),
            "a result event receipt carried a promotion claim"
        )
    );
    assert_eq!(
        failures(&[wrapper::Hole::ChildResult])?,
        crate::tests::state_coverage_let_through(
            passed(&[(NON_SELECTION, RESULT_CLAIM), (RECORDED, RESULT_CLAIM)]),
            "a result event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Claims on receipts projected while the campaign runs: safe deletion on a prune or retirement,
/// and operation success on a result projected against the state the shell holds.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.negative]
fn running_state_claims_fail_the_claims_fixtures() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::RunningDeletion])?,
        crate::tests::state_coverage_let_through(
            passed(&[
                (
                    NON_SELECTION,
                    "a prune event receipt carried a promotion claim"
                ),
                (
                    RECORDED,
                    "a retirement event receipt carried a promotion claim"
                ),
            ]),
            "a prune event receipt carried a promotion claim"
        )
    );
    assert_eq!(
        failures(&[wrapper::Hole::HeldRunningResult])?,
        crate::tests::state_coverage_let_through(
            passed(&[
                (NON_SELECTION, RESULT_CLAIM),
                (PLANNER, RESULT_CLAIM),
                (RECORDED, RESULT_CLAIM),
            ]),
            "a result event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// A consumer that reports the search complete as soon as Campaign plans the frontier stop of a
/// frontier whose every candidate was selected and retired, before the stop is recorded.
#[test]
// r[verify campaign.stop.non_claim]
// r[verify campaign.conformance.claims]
fn an_unrecorded_searched_frontier_claim_fails_the_recorded_stop_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::UnrecordedSearchedFrontier])?,
        crate::tests::state_coverage_let_through(
            passed(&[(
                RECORDED,
                "a frontier-exhausted stop receipt of a searched frontier carried a promotion claim",
            )]),
            "a planner stop's receipt planned before recording carried a promotion claim"
        )
    );
    Ok(())
}
