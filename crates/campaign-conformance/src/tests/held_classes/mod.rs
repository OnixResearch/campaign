//! Eighth closeout-audit loophole adapters kept as negative controls: each shell or mapping that
//! once passed every fixture because the suite presented forks only through two idle or
//! outstanding held projections, one-event rewinds, stop-headed forks, forks capped by record
//! room, wait and stop receipts only from ranked requests, no same-batch refusal, no family decided
//! at an interior level, explore draws below `2^53`, and finding-bearing results only in separate
//! campaigns now fails exactly the named fixtures that present the missing class, with their
//! declared verdicts, under the default and the reordered profile.

mod claims;
mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::EighthAuditAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

/// Failures under `profile` and under its variant with `exploration_order = [1]` and
/// `retention_rank_order = [1]`, which must agree.
fn failures_of(
    holes: &[wrapper::Hole],
    mut profile: crate::inputs::SuiteProfile,
) -> crate::verdicts::ConformanceResult<Failures> {
    let default = failures_under(holes, &profile);
    profile.exploration_order = alloc::vec![1];
    profile.retention_rank_order = alloc::vec![1];
    profile.validate()?;
    assert_eq!(failures_under(holes, &profile), default, "reordered");
    Ok(default)
}

/// Failures under the default profile and under the reordered profile, which must agree.
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    failures_of(holes, super::default_profile()?)
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

/// The load and cache matrices letting through the named forbidden effects of loading and of
/// using a cache.
fn matrices((loaded, cached): (&'static str, &'static str)) -> Failures {
    alloc::vec![
        passed(("history.load_matrix", loaded)),
        passed(("caches.validation_matrix", cached)),
    ]
}

/// `matrices` with the state-coverage fixture letting `forbidden` through.
fn matrices_let_through(names: (&'static str, &'static str), forbidden: &'static str) -> Failures {
    crate::tests::state_coverage_let_through(matrices(names), forbidden)
}

/// Forbidden effects of a shell that loads a rewind through a held prefix of the held-lineage
/// matrix.
const REWOUND: (&str, &str) = (
    "a branch rewound behind the held head was loaded as current history",
    "a cache of a branch rewound behind the held head was used as current state",
);

/// Forbidden effects of a shell that loads, through a held prefix, the held lineage on a graph
/// older than the held projection's (the first refusal after the honest head).
const STALE_GRAPH: (&str, &str) = (
    "the held lineage on a graph older than the held projection's was loaded as current",
    "a cache of the held lineage on a graph older than the held projection's was used as current \
     state",
);

/// Forbidden effects of a shell that loads a catalog fork dropping the held head (H1).
const FORKED: (&str, &str) = (
    "a branch moved onto a fork that drops the held head was loaded as a fast-forward",
    "a cache of a fork that drops the held head was used as current state",
);

/// A valid profile whose candidate and guidance-record bounds (8 and 12) once capped the catalog
/// fork depth at 23 events.
fn small_record_profile() -> crate::verdicts::ConformanceResult<crate::inputs::SuiteProfile> {
    let mut small = super::default_profile()?;
    small.bounds.max_candidates = 8;
    small.bounds.max_candidate_batch = 8;
    small.bounds.max_retained_candidates = 8;
    small.bounds.max_prune_batch = 8;
    small.bounds.max_guidance_records = 12;
    small.bounds = small.bounds.clone().derive()?;
    small.validate()?;
    Ok(small)
}

/// Control events of the whole held lineage: H1's six, a result, a selection, a closure, two
/// selections, a stop, and a post-stop result.
const HELD_LINEAGE: usize = 13;

/// Events the deepest catalog fork over H0 reaches beyond H1 under the presented projection bound
/// (H1 holds six control events).
const DEEPEST_GAP: usize = 58;

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    assert_eq!(failures_of(&[], small_record_profile()?)?, Failures::new());
    Ok(())
}

/// A shell that reloads cold once its held campaign is stopped, or once it settled a selection,
/// loads, through the stopped or settled prefix, its lineage on a graph older than the held one.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn cold_when_stopped_or_settled_shells_fail_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    for hole in [
        wrapper::Hole::ColdWhenStopped,
        wrapper::Hole::ColdWhenSettled,
    ] {
        assert_eq!(
            failures(&[hole])?,
            matrices_let_through(
                STALE_GRAPH,
                "a held state's lineage on an older graph was loaded as current"
            ),
            "{hole:?}"
        );
    }
    Ok(())
}

/// A shell that reloads cold once its held lineage is longer than H1 (six events), or longer
/// than every prefix but the last two of the held lineage, loads through a longer prefix that
/// prefix's lineage on a graph older than the held one.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn cold_beyond_prefix_length_shells_fail_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    for events in [6, HELD_LINEAGE.saturating_sub(2)] {
        assert_eq!(
            failures(&[wrapper::Hole::ColdWhenHeldLongerThan { events }])?,
            matrices_let_through(
                STALE_GRAPH,
                "a held state's lineage on an older graph was loaded as current"
            ),
            "held longer than {events}"
        );
    }
    Ok(())
}

/// A shell that treats a rewind of two events as a reset loads a branch rewound past an executed
/// selection.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_two_event_rewind_resync_shell_fails_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::RewindResync { depth: 2 }])?,
        matrices_let_through(
            REWOUND,
            "a branch rewound behind a held state was loaded as current history"
        )
    );
    Ok(())
}

/// A shell that reloads cold unless a lineage at least as long as the held one ends in a stop
/// loads a same-depth fork of H1 whose head is not a stop, and the state coverage's first
/// selected state (running) on the graph before its later moment.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_cold_unless_stop_head_shell_fails_the_load_and_cache_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ColdUnlessStopHead])?,
        matrices_let_through(
            FORKED,
            "a held state's lineage on an older graph was loaded as current"
        )
    );
    Ok(())
}

/// Under candidate and guidance-record bounds that once capped the fork fill at 23 events, the
/// fill continues with coalesced duplicates: shells resyncing 24 events ahead and at the deepest
/// gap the projection bound admits load a fork that drops H1.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn gap_resync_shells_beyond_the_record_room_fail_under_small_record_bounds()
-> crate::verdicts::ConformanceResult<()> {
    for gap in [24, DEEPEST_GAP] {
        assert_eq!(
            failures_of(&[wrapper::Hole::GapResync { gap }], small_record_profile()?)?,
            matrices_let_through(FORKED, crate::tests::FORK_THROUGH_HELD),
            "gap {gap}"
        );
    }
    Ok(())
}

/// A guidance mapping that zeroes every level from the third to the second-to-last lets the last
/// level decide ([5, 5, 9, 1] against [5, 5, 1, 9] selects B instead of A).
#[test]
// r[verify campaign.guidance.valid]
fn zeroed_interior_levels_fail_the_interior_level_fixture() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        failures(&[wrapper::Hole::ZeroInteriorLevels])?,
        alloc::vec![diverged((
            "selection.interior_level",
            "an interior level decides between candidates whose leading levels tie"
        ))]
    );
    Ok(())
}

/// A ticket mapping that carries the explore draw through a binary64 float issues tickets that
/// do not decode to the explore draws of at least `2^53`.
#[test]
// r[verify campaign.entropy.valid]
fn binary64_explore_draws_fail_the_ticket_decoding() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::Binary64ExploreDraw])?,
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

/// A result mapping that keys findings by their position in the result reports the second
/// finding-bearing result of one campaign under the first result's finding identities.
#[test]
// r[verify campaign.findings.valid]
fn position_keyed_findings_fail_the_findings_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::FindingsKeyedByPosition])?,
        alloc::vec![diverged((
            "results.findings",
            "the result reports every observed finding under its own identity"
        ))]
    );
    Ok(())
}
