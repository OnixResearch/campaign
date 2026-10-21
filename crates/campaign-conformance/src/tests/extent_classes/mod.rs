//! Sixth closeout-audit loophole adapters kept as negative controls: each shell or mapping that
//! once passed every fixture because the suite presented one fixed list length, fork depths up
//! to a named bound, or one settlement order, now fails exactly the named fixtures that present
//! the missing class, with their declared verdicts. The presented classes: forks one to three
//! events deeper than a held head and the deepest fork the bounds admit, over H0 (dropping H1)
//! and over the history before H0's head (dropping H0), deepened past the guidance room;
//! scores of one level, three levels, and the profile's most rank dimensions, tied on every level
//! but the last; results with one child, three children, and the most one result may carry;
//! results with one finding, four findings, and the most one result may carry; and two in-flight
//! executions of one candidate settled newer first.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

/// Forbidden effect of a result filed under another in-flight selection.
const CROSSED: &str = "a result settled a selection other than the executed one";

/// Forbidden effects of loading, and of using a cache of, a fork that drops the held head.
const DEEPER_FORK_LOADED: &str =
    "a branch moved onto a fork that drops the held head was loaded as a fast-forward";
const DEEPER_FORK_CACHED: &str =
    "a cache of a fork that drops the held head was used as current state";

/// Forbidden effects of loading, and of using a cache of, a fork of the history before the
/// earlier held head through that head.
const EARLIER_FORK_LOADED: &str =
    "a branch moved onto a fork that drops the earlier held head was loaded as a fast-forward";
const EARLIER_FORK_CACHED: &str =
    "a cache of a fork that drops the earlier held head was used as current state";

/// Largest named fork depth the catalog presents besides the maximal depth.
const NAMED_FORK_DEPTH: usize = 3;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::UnsampledLengthAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    Ok(failures_under(holes, &super::default_profile()?))
}

/// One positive fixture diverged from its named expectation.
fn diverged(
    (name, expected): (&'static str, &'static str),
) -> (&'static str, crate::verdicts::FixtureVerdict) {
    (
        name,
        crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
            crate::verdicts::ConformanceError::Diverged { expected },
        ),
    )
}

/// One negative fixture let through the named forbidden effect.
const fn passed(
    (name, forbidden): (&'static str, &'static str),
) -> (&'static str, crate::verdicts::FixtureVerdict) {
    (
        name,
        crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
    )
}

/// Control events of the held projections the history catalog presents forks against: H1 (the
/// admission, one candidate batch, three guidance records, and one published selection) and
/// H0 (one event older).
fn held_lineages() -> crate::verdicts::ConformanceResult<(usize, usize)> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 64, |_| {})?;
    let operations: alloc::vec::Vec<crate::vocabulary::ScenarioOperation> = (1..=3)
        .map(|handle| crate::vocabulary::ScenarioOperation {
            handle: crate::vocabulary::OperationHandle(handle),
            parent: crate::harness::history::ROOT,
            costs: alloc::vec![crate::vocabulary::AbstractCost {
                dimension: 0,
                units: 1,
            }],
        })
        .collect();
    session.admit(&operations)?;
    for (handle, levels) in [(1, [30, 1]), (2, [20, 3]), (3, [10, 2])] {
        session.guide(&crate::vocabulary::ScenarioPreference {
            operation: crate::vocabulary::OperationHandle(handle),
            generation: 1,
            score: crate::vocabulary::AbstractScore {
                levels: levels.to_vec(),
            },
        })?;
    }
    let earlier = session.payloads.len();
    let selection = session.execute(None)?;
    assert_eq!(selection.ordinal, 1);
    assert_eq!(session.state.guidance_record_count, 3);
    Ok((earlier, session.payloads.len()))
}

/// The deepest forks the presented projection bound admits beyond H1 and beyond H0.
fn maximal_fork_depths() -> crate::verdicts::ConformanceResult<(usize, usize)> {
    let bound = usize::try_from(crate::harness::HISTORY_MAX_EVENTS)
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let (earlier, current) = held_lineages()?;
    Ok((bound.saturating_sub(current), bound.saturating_sub(earlier)))
}

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    let mut reordered = super::default_profile()?;
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    assert_eq!(failures_under(&[], &reordered), Failures::new());
    Ok(())
}

/// A correct consumer passes every fixture under the smallest list bounds the suite admits: the
/// most children, findings, and rank levels one result or score carries are the profile's.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.positive]
fn the_smallest_list_bounds_pass_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    let mut profile = super::default_profile()?;
    profile.bounds.max_child_moments = 2;
    profile.bounds.max_findings = 3;
    profile.bounds.max_rank_dimensions = 2;
    profile.bounds = profile.bounds.clone().derive()?;
    assert_eq!(profile.validate(), Ok(()));
    assert_eq!(failures_under(&[], &profile), Failures::new());
    Ok(())
}

/// A shell that reloads cold once the presented lineage is some number of events ahead of its
/// held projection loads a fork that deep and validates its cache: for one event beyond the
/// named fork depths and the deepest fork beyond H1, a fork over H0 through H1; for the deepest
/// fork beyond H0, a fork over the history before H0's head through H0.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn gap_resync_shells_up_to_the_projection_bound_fail_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    let (beyond_current, beyond_earlier) = maximal_fork_depths()?;
    assert!(beyond_current > NAMED_FORK_DEPTH.saturating_add(1));
    assert_eq!(beyond_earlier, beyond_current.saturating_add(1));
    for gap in [NAMED_FORK_DEPTH.saturating_add(1), beyond_current] {
        assert_eq!(
            failures(&[wrapper::Hole::GapResync { gap }])?,
            crate::tests::state_coverage_let_through(
                [
                    passed(("history.load_matrix", DEEPER_FORK_LOADED)),
                    passed(("caches.validation_matrix", DEEPER_FORK_CACHED)),
                ],
                crate::tests::FORK_THROUGH_HELD
            ),
            "gap {gap}"
        );
    }
    assert_eq!(
        failures(&[wrapper::Hole::GapResync {
            gap: beyond_earlier
        }])?,
        crate::tests::state_coverage_let_through(
            [
                passed(("history.load_matrix", EARLIER_FORK_LOADED)),
                passed(("caches.validation_matrix", EARLIER_FORK_CACHED)),
            ],
            crate::tests::FORK_THROUGH_HELD
        )
    );
    Ok(())
}

/// Under the smallest guidance-record bound the suite admits, forks deepen past the guidance
/// room with single-candidate batches and retirements: a shell that resyncs ten events ahead of
/// its held projection loads a fork over H0 through H1.
#[test]
// r[verify campaign.conformance.bounds]
fn a_gap_resync_shell_beyond_the_guidance_room_fails_under_the_smallest_guidance_bound()
-> crate::verdicts::ConformanceResult<()> {
    let mut profile = super::default_profile()?;
    profile.bounds.max_guidance_records = 12;
    profile.bounds = profile.bounds.clone().derive()?;
    assert_eq!(profile.validate(), Ok(()));
    assert_eq!(
        failures_under(&[wrapper::Hole::GapResync { gap: 10 }], &profile),
        crate::tests::state_coverage_let_through(
            [
                passed(("history.load_matrix", DEEPER_FORK_LOADED)),
                passed(("caches.validation_matrix", DEEPER_FORK_CACHED)),
            ],
            crate::tests::FORK_THROUGH_HELD
        )
    );
    Ok(())
}

/// A guidance mapping that keeps two levels of every score ties three-level and
/// most-dimension scores whose last level decides.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn two_level_scores_fail_the_level_count_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::TwoLevelScores])?,
        alloc::vec![
            diverged((
                "selection.level_counts",
                "the last level decides between candidates whose leading levels tie"
            )),
            diverged((
                "selection.interior_level",
                "an improved interior level decides between candidates whose leading levels tie"
            )),
        ]
    );
    Ok(())
}

/// A result mapping that reports at most two children drops a reserved child moment of a result
/// with three or more.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn capped_children_fail_the_child_count_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CappedChildren])?,
        alloc::vec![diverged((
            "results.child_counts",
            "the result reports and the state records every discovered child"
        ))]
    );
    Ok(())
}

/// A result mapping that reports at most three findings drops a finding of a result with four or
/// more.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn capped_findings_fail_the_findings_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CappedFindings])?,
        alloc::vec![diverged((
            "results.findings",
            "the result reports every observed finding under its own identity"
        ))]
    );
    Ok(())
}

/// A result mapping that files each outcome under the candidate's oldest in-flight selection
/// settles the older selection with the newer execution's result when the newer one finishes
/// first.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn oldest_in_flight_results_fail_the_out_of_order_settlement_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::OldestInFlight])?,
        alloc::vec![passed(("results.out_of_order_settlement", CROSSED))]
    );
    Ok(())
}
