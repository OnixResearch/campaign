//! Seventh closeout-audit loophole adapters kept as negative controls: each shell or mapping that
//! once passed every fixture because the suite presented one level order, one outstanding
//! selection per closed candidate, only narrow eligible counts without decoding tickets, or no
//! fork dropping the earlier held head now fails exactly the named fixtures that present the
//! missing class, with their declared verdicts, under the default and the reordered profile. The
//! presented classes: guidance whose middle level decides and whose later level exceeds an
//! earlier one; two in-flight selections of one candidate closed in both orders; tickets decoded
//! against the scenario draw and seven eligible candidates; and forks over the history before
//! the earlier held head.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::UnsampledOrderAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

/// Failures under the default profile and under the reordered profile, which must agree.
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let default = failures_under(holes, &super::default_profile()?);
    let mut reordered = super::default_profile()?;
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    assert_eq!(failures_under(holes, &reordered), default, "reordered");
    Ok(default)
}

/// One positive fixture diverged from its named expectation.
const fn diverged(
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

/// Forbidden effects of loading, and of using a cache of, a fork that drops the earlier held
/// head.
const EARLIER_FORK: (&str, &str) = (
    "a branch moved onto a fork that drops the earlier held head was loaded as a fast-forward",
    "a cache of a fork that drops the earlier held head was used as current state",
);

/// First failure of the interior-level family.
const INTERIOR_LEVEL: (&str, &str) = (
    "selection.interior_level",
    "an interior level decides between candidates whose leading levels tie",
);

/// First failure of the middle-level family.
const MIDDLE_LEVEL: (&str, &str) = (
    "selection.middle_level",
    "a middle level decides between candidates whose first levels tie",
);

#[test]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A guidance mapping that zeroes every middle level of a score of three or more levels lets
/// the last level decide ([5, 9, 1] against [5, 1, 9] selects B instead of A, and likewise for
/// the interior-level family).
#[test]
// r[verify campaign.conformance.bounds]
fn zeroed_middle_levels_fail_the_middle_and_interior_level_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ZeroMiddleLevels])?,
        alloc::vec![diverged(MIDDLE_LEVEL), diverged(INTERIOR_LEVEL)]
    );
    Ok(())
}

/// A guidance mapping that sorts every score's levels largest first reorders the middle-level and
/// interior-level families and the rising family ([1, 30] against [20, 3] selects A instead of
/// B).
#[test]
// r[verify campaign.conformance.bounds]
fn sorted_levels_fail_the_level_family_and_level_order_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::SortLevelsDescending])?,
        alloc::vec![
            diverged(MIDDLE_LEVEL),
            diverged(INTERIOR_LEVEL),
            diverged((
                "selection.level_order",
                "the first level decides although a later level is larger"
            )),
        ]
    );
    Ok(())
}

/// A closure mapping keyed by candidate closes the newer of two in-flight executions when the
/// older is closed first, and closes a selection a fork never published when a fork writer closes
/// the fork's outstanding selection (Campaign refuses the closure while the load and cache
/// matrices and the state coverage build their forks).
#[test]
// r[verify campaign.conformance.bounds]
fn closures_keyed_by_candidate_fail_the_closure_and_fork_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let missing = || {
        crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
            crate::verdicts::ConformanceError::Campaign(
                campaign_core::CampaignError::MissingSelection,
            ),
        )
    };
    let failed = failures(&[wrapper::Hole::ClosureKeyedByCandidate])?;
    assert_eq!(failed.len(), 4);
    assert_eq!(
        failed,
        crate::tests::with_state_coverage(
            alloc::vec![
                passed((
                    "closures.out_of_order",
                    crate::harness::settlement::CROSSED_CLOSURE
                )),
                ("history.load_matrix", missing()),
                ("caches.validation_matrix", missing()),
            ],
            missing()
        )
    );
    Ok(())
}

/// A ticket mapping that reduces both draws modulo one million issues tickets that do not
/// decode to the wide and exploit-side scenario draws.
#[test]
// r[verify campaign.conformance.bounds]
fn parts_per_million_tickets_fail_the_ticket_decoding() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::NarrowIndexDraw])?,
        super::undecoded(&[
            (
                "entropy.exploit",
                crate::verdicts::FixturePolarity::Positive
            ),
            (
                "entropy.wide_draws",
                crate::verdicts::FixturePolarity::Positive
            ),
        ])
    );
    Ok(())
}

/// A shell that reloads cold whenever its held projection has nothing outstanding loads,
/// through H0, a fork of the history before H0's head.
#[test]
// r[verify campaign.conformance.bounds]
fn a_cold_when_idle_shell_fails_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ColdWhenIdle])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![
                passed(("history.load_matrix", EARLIER_FORK.0)),
                passed(("caches.validation_matrix", EARLIER_FORK.1)),
            ],
            "a held state's lineage on an older graph was loaded as current"
        )
    );
    Ok(())
}
