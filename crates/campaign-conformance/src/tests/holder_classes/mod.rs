//! Ninth closeout-audit loophole adapters kept as negative controls: each shell that once passed
//! every fixture because the held matrix presented deep forks and the forged and suffix catalog
//! only through idle and outstanding held projections, never advanced a stopped held projection,
//! and no recorded frontier or budget stop reached the claims oracle, now fails exactly the named
//! fixtures that present the missing class, with their declared verdicts, under the default and
//! the reordered profile.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::NinthAuditAdapter::new(holes);
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

/// One negative fixture let through the named forbidden effect.
const fn passed(
    (name, forbidden): (&'static str, &'static str),
) -> (&'static str, crate::verdicts::FixtureVerdict) {
    (
        name,
        crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
    )
}

/// The load and cache matrices letting through the named forbidden effects of loading and of
/// using a cache.
fn matrices((loaded, cached): (&'static str, &'static str)) -> Failures {
    alloc::vec![
        passed(("history.load_matrix", loaded)),
        passed(("caches.validation_matrix", cached)),
    ]
}

/// First failures of a shell that loads, through a settled held prefix, a fork of the history
/// before the held head: the load and cache matrices, and the state coverage's forks through its
/// selected states.
fn forked() -> Failures {
    crate::tests::state_coverage_let_through(
        matrices((
            "a branch moved onto a fork of the history before the held head was loaded as a \
             fast-forward",
            "a cache of a fork of the history before the held head was used as current state",
        )),
        crate::tests::FORK_THROUGH_HELD,
    )
}

/// Events the deepest fork through a settled held prefix reaches beyond it: through the prefix
/// that settles H1's selection by its result (seven events), a stop-headed fork of H1's history
/// (six events, one selection outstanding) fills the projection bound of 64 less that selection's
/// settlement reserve.
const DEEPEST_SETTLED_GAP: usize = 56;

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A shell that resyncs cold once its held campaign settled or stopped and fell `gap` events
/// behind loads, through the settled prefix, a fork of the history before the held head: at a
/// gap of three (a named depth), sixteen, and the deepest gap through a settled prefix.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn gap_resync_after_settlement_shells_fail_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    for gap in [3, 16, DEEPEST_SETTLED_GAP] {
        assert_eq!(
            failures(&[wrapper::Hole::GapResyncAfterSettlement { gap }])?,
            forked(),
            "gap {gap}"
        );
    }
    Ok(())
}

/// No lineage the held-lineage matrix presents is more than the deepest settled gap ahead of a
/// settled or stopped held prefix, but the state coverage forks the history before every stopped
/// or settled selected state's head as deep as the bounds admit, so a shell whose resync gap
/// exceeds the settled gap fails the state coverage alone.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.state_coverage]
fn a_gap_resync_after_settlement_shell_beyond_the_deepest_settled_gap_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let gap = DEEPEST_SETTLED_GAP.saturating_add(1);
    assert_eq!(
        failures(&[wrapper::Hole::GapResyncAfterSettlement { gap }])?,
        crate::tests::state_coverage_let_through([], crate::tests::FORK_THROUGH_HELD)
    );
    Ok(())
}

/// A shell that keeps serving its settled or stopped held projection on any refusal but a
/// non-fast-forward answers, through the first settled prefix, its lineage on an older graph
/// with the held projection; spared that refusal too, it answers the renamed branch, the first
/// catalog class presented through the prefix, with the held projection.
#[test]
// r[verify campaign.conformance.negative]
// r[verify campaign.history_projection.stale]
fn last_good_when_settled_shells_fail_the_load_matrix() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::LastGoodWhenSettled {
            spares_stale_graph: false
        }])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "history.load_matrix",
                "the held lineage on a graph older than the held projection's was loaded as current",
            ))],
            "a held state's lineage on an older graph was loaded as current"
        )
    );
    assert_eq!(
        failures(&[wrapper::Hole::LastGoodWhenSettled {
            spares_stale_graph: true
        }])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "history.load_matrix",
                "a renamed control branch was loaded as the campaign's history",
            ))],
            "a renamed control branch was loaded as the campaign's history"
        )
    );
    Ok(())
}

/// A shell that treats a stopped campaign as final answers the honest lineage, which settles a
/// selection outstanding at the stop, with the stopped held projection instead of Campaign's; the
/// non-selection claims fixture, which closes its explored selection after the consumer stop,
/// diverges the same way.
#[test]
// r[verify campaign.conformance.negative]
// r[verify campaign.history_projection.stale]
fn a_stopped_projection_is_final_shell_fails_the_load_matrix_and_the_post_stop_claims()
-> crate::verdicts::ConformanceResult<()> {
    let failed = failures(&[wrapper::Hole::StoppedHolderIsFinal])?;
    assert_eq!(failed.len(), 3);
    assert_eq!(
        failed,
        crate::tests::with_state_coverage(
            alloc::vec![
                (
                    "history.load_matrix",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                        crate::verdicts::ConformanceError::Diverged {
                            expected: "the adapter's shell returns Campaign's own answer",
                        }
                    ),
                ),
                (
                    "receipts.non_selection_overclaim",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                        crate::verdicts::ConformanceError::Diverged {
                            expected: "the loaded projection binds the session's own graph and branch",
                        }
                    ),
                ),
            ],
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "the adapter's shell returns Campaign's own answer"
                }
            )
        )
    );
    Ok(())
}

/// A consumer that reports the search complete once Campaign records a frontier or budget stop
/// claims exhaustive search on the first receipt projected against the recorded frontier stop's
/// state, the admission's event receipt; keyed on the event receipts of recorded budget stops
/// only, it claims it on the budget stop's event receipt.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
// r[verify campaign.conformance.claims]
fn recorded_exhaustion_claims_fail_the_recorded_stop_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::RecordedExhaustionClaim])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "receipts.recorded_stop_overclaim",
                "a campaign admission event receipt carried a promotion claim",
            ))],
            "a campaign admission event receipt carried a promotion claim"
        )
    );
    assert_eq!(
        failures(&[wrapper::Hole::RecordedBudgetEventClaim])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "receipts.recorded_stop_overclaim",
                "a budget-exhausted stop event receipt carried a promotion claim",
            ))],
            "a budget-exhausted stop event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Control events of the shortest held prefix the held-lineage matrix presents forks through: the
/// admission and the candidate batch.
const SHORTEST_FORKED_HOLDER: usize = 2;

/// The deepest stop-headed fork through the shortest forked held prefix fills the projection
/// bound: a shell resyncing at that gap loads it through the prefix, and one resyncing a gap
/// beyond it answers every presentation as Campaign does.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.history_projection.stale]
fn a_gap_resync_shell_meets_the_projection_bound_through_the_shortest_forked_prefix()
-> crate::verdicts::ConformanceResult<()> {
    let bound = usize::try_from(crate::harness::HISTORY_MAX_EVENTS)
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let gap = bound.saturating_sub(SHORTEST_FORKED_HOLDER);
    assert_eq!(failures(&[wrapper::Hole::GapResync { gap }])?, forked());
    assert_eq!(
        failures(&[wrapper::Hole::GapResync {
            gap: gap.saturating_add(1)
        }])?,
        Failures::new()
    );
    Ok(())
}
