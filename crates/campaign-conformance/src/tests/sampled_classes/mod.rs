//! Fifth closeout-audit loophole adapters kept as negative controls: each shell or mapping that
//! once passed every fixture because the suite never presented its input class now fails exactly
//! the named fixtures that present it, with their declared verdicts, in debug and release
//! builds. The presented classes: non-selection publications with their own committed evidence
//! in both gate matrices; forks one to three events deeper than the held head and every catalog
//! class through the older held projection; cost units above `2^53` and near `u64::MAX`; index
//! and explore draws above `u32::MAX` on both sides of the threshold; exploit-side draws whose
//! exploration position differs from the ranked choice under any declared exploration order;
//! a lineage longer than the projection bound; and a control root under foreign envelope
//! identities. Suite choices among candidates follow the
//! suite's operation order, so consumers whose candidate identities order differently agree on
//! every decision.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

/// Expectation of the declared-dimension fixture.
const DECLARED_COSTS: &str =
    "the adapter declares every abstract cost in its own dimension with its exact units";

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::UnpresentedClassAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    Ok(failures_under(holes, &super::default_profile()?))
}

/// The default profile with dimension one declared as both the exploration and the retention
/// order: the exploration order puts the ranked choice last.
fn reordered_profile() -> crate::verdicts::ConformanceResult<crate::inputs::SuiteProfile> {
    let mut profile = super::default_profile()?;
    profile.exploration_order = alloc::vec![1];
    profile.retention_rank_order = alloc::vec![1];
    Ok(profile)
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

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    assert_eq!(failures_under(&[], &reordered_profile()?), Failures::new());
    Ok(())
}

/// A correct consumer that declares two resource dimensions and maps every cost exactly passes
/// every fixture, including costs near `u64::MAX` in both dimensions at once.
#[test]
// r[verify campaign.conformance.positive]
fn a_correct_second_dimension_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CorrectSecondDimension])?,
        Failures::new()
    );
    Ok(())
}

/// A correct consumer passes every fixture under restrictive but valid profiles, among them the
/// smallest threshold and the largest threshold below the entropy scale (the generated wide
/// explore draws take both construction branches).
#[test]
// r[verify campaign.conformance.positive]
fn restrictive_valid_profiles_pass_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    let edits: [fn(&mut crate::inputs::SuiteProfile); 5] = [
        |profile| profile.bounds.max_outstanding = 2,
        |profile| profile.bounds.max_candidate_batch = 3,
        |profile| profile.bounds.max_selections = 8,
        |profile| profile.exploratory_threshold_parts_per_million = 2,
        |profile| profile.exploratory_threshold_parts_per_million = 999_999,
    ];
    for edit in edits {
        let mut profile = super::default_profile()?;
        edit(&mut profile);
        profile.bounds = profile.bounds.clone().derive()?;
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(failures_under(&[], &profile), Failures::new());
    }
    Ok(())
}

/// A gate that trusts a non-selection publication's own committed evidence grants a selection
/// that was never published.
#[test]
// r[verify campaign.conformance.negative]
fn a_non_selection_publication_gate_fails_both_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    let forbidden = "a selection executed under a guidance publication's committed evidence";
    assert_eq!(
        failures(&[wrapper::Hole::NonSelectionPublicationGate])?,
        alloc::vec![
            passed(("fencing.gate_matrix_ranked", forbidden)),
            passed(("fencing.gate_matrix_exploratory", forbidden)),
        ]
    );
    Ok(())
}

/// A shell that reloads cold once the presented lineage is two events ahead of its held
/// projection loads a fork two events deeper than the held head, and validates its cache.
#[test]
// r[verify campaign.conformance.negative]
fn a_gap_resync_shell_fails_the_load_and_cache_matrices() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(wrapper::RESYNC_GAP, 2);
    assert_eq!(
        failures(&[wrapper::Hole::GapResyncHistory])?,
        crate::tests::state_coverage_let_through(
            [
                passed((
                    "history.load_matrix",
                    "a branch moved onto a fork that drops the held head was loaded as a \
                     fast-forward"
                )),
                passed((
                    "caches.validation_matrix",
                    "a cache of a fork that drops the held head was used as current state"
                )),
            ],
            crate::tests::FORK_THROUGH_HELD
        )
    );
    Ok(())
}

/// Cost units carried through an `f64` mantissa declare and reserve `2^53` for `2^53 + 1`.
#[test]
// r[verify campaign.conformance.negative]
fn mantissa_cost_units_fail_the_declared_dimensions_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::MantissaCostUnits])?,
        alloc::vec![diverged(("budgets.declared_dimensions", DECLARED_COSTS))]
    );
    Ok(())
}

/// Fixtures whose draws are wide or on the exploit side, where the ticket mappings below issue a
/// ticket that does not decode to the scenario draw.
const WIDE_OR_EXPLOITING: [(&str, crate::verdicts::FixturePolarity); 2] = [
    (
        "entropy.exploit",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "entropy.wide_draws",
        crate::verdicts::FixturePolarity::Positive,
    ),
];

/// Draws carried through `u32` lose the high words of wide draws: the exploit and wide-draw
/// fixtures fail at the ticket's decoding.
#[test]
// r[verify campaign.conformance.negative]
fn u32_draws_fail_the_exploit_and_wide_draw_ticket_decoding()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::U32Draws])?,
        super::undecoded(&WIDE_OR_EXPLOITING)
    );
    Ok(())
}

/// A ticket mapping that always explores rewrites exploit-side draws: the exploit and wide-draw
/// fixtures fail at the ticket's decoding under the default profile and under an exploration
/// order that puts the ranked choice last.
#[test]
// r[verify campaign.conformance.negative]
fn always_exploring_fails_the_ticket_decoding_under_every_order()
-> crate::verdicts::ConformanceResult<()> {
    let expected = super::undecoded(&WIDE_OR_EXPLOITING);
    assert_eq!(failures(&[wrapper::Hole::AlwaysExplore])?, expected);
    assert_eq!(
        failures_under(&[wrapper::Hole::AlwaysExplore], &reordered_profile()?),
        expected
    );
    Ok(())
}

/// A shell that serves its held projection when the lineage outgrows the projection bound
/// loads a stale projection of a branch that has moved on.
#[test]
// r[verify campaign.conformance.negative]
fn holding_on_the_projection_limit_fails_the_load_matrix() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        failures(&[wrapper::Hole::HeldOnProjectionLimit])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "history.load_matrix",
                "a control lineage longer than the projection bound was loaded as history"
            ))],
            "a control lineage longer than the projection bound was loaded as history"
        )
    );
    Ok(())
}

/// A shell that serves its held projection when the presented envelope identities cross the
/// admitted ones loads a control root under foreign envelope identities as the campaign's
/// history.
#[test]
// r[verify campaign.conformance.negative]
fn holding_on_crossed_envelope_identities_fails_the_load_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::HeldOnCrossedIdentity])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![passed((
                "history.load_matrix",
                "a control root under foreign envelope identities was loaded as the campaign's \
             history"
            ))],
            "a control root under foreign envelope identities was loaded as the campaign's history"
        )
    );
    Ok(())
}

/// Consumers whose candidate identities order differently from the healthy adapter's record
/// exactly the healthy adapter's transcript under the default and a reordered profile: every
/// suite choice among candidates follows the suite's operation order.
#[test]
// r[verify campaign.conformance.positive]
fn relabelled_candidate_identities_record_the_same_transcript()
-> crate::verdicts::ConformanceResult<()> {
    for profile in [super::default_profile()?, reordered_profile()?] {
        let healthy = crate::catalog::run_suite(
            &mut crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            &profile,
        );
        for salt in 0..8 {
            let relabelled = crate::catalog::run_suite(
                &mut wrapper::UnpresentedClassAdapter::new(&[wrapper::Hole::Relabelled(salt)]),
                &profile,
            );
            assert_eq!(super::failures(&relabelled), Failures::new());
            assert_eq!(relabelled.transcript, healthy.transcript);
        }
    }
    Ok(())
}
