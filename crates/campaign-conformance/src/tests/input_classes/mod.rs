//! Third closeout-audit loophole adapters kept as negative controls: each shell that once passed
//! every fixture because the suite never presented its input class (single-field gate evidence,
//! a fork, planner stops and waits, unpresented load and cache denials, cold forged
//! histories, several findings) now fails exactly the named fixtures that close it, with their
//! declared verdicts, in debug and release builds. A shell that ignores a cache request's held
//! projection fails the cache oracle.

mod wrapper;

/// Fixture name and declared forbidden effect of one negative fixture.
type Declared = (&'static str, &'static str);

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let profile = super::default_profile()?;
    let mut adapter = wrapper::HoledAdapter::new(holes);
    Ok(super::failures(&crate::catalog::run_suite(
        &mut adapter,
        &profile,
    )))
}

/// Negative fixtures whose declared forbidden input or effect went through.
fn let_through(declared: &[Declared]) -> Failures {
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

#[test]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A gate that checks durability, the published decision, and `branch_before` but ignores the
/// evidence's event identity and branch value after grants evidence naming another event (and a
/// compare-and-set loser under the winner's committed transition).
#[test]
fn a_gate_ignoring_evidence_event_and_after_fails_both_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    let other_event = "evidence naming another event executed the published selection";
    assert_eq!(
        failures(&[wrapper::Hole::SloppyEvidenceGate])?,
        let_through(&[
            ("fencing.gate_matrix_ranked", other_event),
            ("fencing.gate_matrix_exploratory", other_event),
        ])
    );
    Ok(())
}

/// Serving the held projection on recorded-fence, envelope, parent, and Campaign denials.
#[test]
fn a_journal_on_unpresented_load_denials_fails_the_load_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::JournalOnUnpresentedLoadDenials])?,
        crate::tests::state_coverage_let_through(
            let_through(&[(
                "history.load_matrix",
                "a merged control event was loaded as the linear child of the head",
            )]),
            "a merged control event was loaded as the linear child of the head"
        )
    );
    Ok(())
}

/// Keeping only the first finding of a result.
#[test]
fn a_first_finding_only_mapping_fails_the_findings_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::FirstFindingOnly])?,
        alloc::vec![(
            "results.findings",
            crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "the result reports every observed finding under its own identity",
                }
            ),
        )]
    );
    Ok(())
}

/// Claiming exhaustive search on a stop receipt planned while the campaign is not stopped, which
/// the planner and the recorded-stop fixtures present before any stop is recorded.
#[test]
fn an_unrecorded_stop_claim_fails_the_planner_and_recorded_stop_claim_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::UnrecordedStopClaim])?,
        crate::tests::state_coverage_let_through(
            let_through(&[
                (
                    "receipts.planner_overclaim",
                    "a frontier-exhausted stop receipt carried a promotion claim",
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    "a frontier-exhausted stop receipt carried a promotion claim",
                ),
            ]),
            "a planner stop's receipt planned before recording carried a promotion claim"
        )
    );
    Ok(())
}

/// Advancing by cold-start validation plus a length check instead of the prefix check.
#[test]
fn a_length_monotone_advance_fails_the_load_matrix() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::LengthMonotoneAdvance])?,
        crate::tests::state_coverage_let_through(
            let_through(&[(
                "history.load_matrix",
                "a branch moved onto a fork that drops the held head was loaded as a fast-forward",
            )]),
            "a held state's lineage on an older graph was loaded as current"
        )
    );
    Ok(())
}

/// Serving an internal journal on rewritten, altered, ambiguous, or merged histories presented
/// cold.
#[test]
fn a_cold_journal_on_forged_denials_fails_every_forged_history_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let failed = failures(&[wrapper::Hole::ColdJournalOnForgedDenials])?;
    assert_eq!(failed.len(), 4);
    assert_eq!(
        failed,
        let_through(&[
            (
                "history.rewritten_payload",
                "a rewritten control payload was loaded as history",
            ),
            (
                "history.altered_graph",
                "history altered after binding was loaded as current",
            ),
            (
                "history.ambiguous_lineage",
                "an ambiguous control lineage was loaded as history",
            ),
            (
                "history.load_matrix",
                "a control payload rewritten into another replayable event was loaded as history",
            ),
        ])
    );
    Ok(())
}

/// Falling back to core replay of the presented payloads on lineage denials.
#[test]
fn a_cache_fallback_on_lineage_denials_fails_the_cache_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CacheFallbackOnLineageDenials])?,
        crate::tests::state_coverage_let_through(
            let_through(&[(
                "caches.validation_matrix",
                "a cache of a rewound control branch was used as current state",
            )]),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}

/// Validating every cache as a cold start, ignoring the host's held projection.
#[test]
fn a_cache_check_ignoring_the_held_projection_fails_the_cache_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ColdCacheIgnoringHeld])?,
        crate::tests::state_coverage_let_through(
            let_through(&[(
                "caches.validation_matrix",
                "a cache of a rewound control branch was used as current state",
            )]),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}
