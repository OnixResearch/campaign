//! Fourth closeout-audit loophole adapters kept as negative controls: each shell or mapping that
//! once passed every fixture because the suite sampled its input class by hand now fails exactly
//! the named fixtures whose generated classes close it, with their declared verdicts, in debug
//! and release builds. The generators: every single-field tampering of a published decision,
//! every single-field mutation of its durable evidence, every committed session transition
//! presented for another publication, one shared history catalog presented to the load and
//! cache oracles cold and held (with an honest held path older than the head), costs in every
//! declared dimension above `u32::MAX` units, index draws in every residue of each eligible
//! count, and guidance whose first levels tie.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let profile = super::default_profile()?;
    let mut adapter = wrapper::GeneratedClassAdapter::new(holes);
    Ok(super::failures(&crate::catalog::run_suite(
        &mut adapter,
        &profile,
    )))
}

/// Both gate matrices let through the named forbidden execution.
fn both_gate_matrices(forbidden: &'static str) -> Failures {
    [
        "fencing.gate_matrix_ranked",
        "fencing.gate_matrix_exploratory",
    ]
    .into_iter()
    .map(|name| {
        (
            name,
            crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
        )
    })
    .collect()
}

/// One positive fixture diverged from its named expectation.
fn diverged((name, expected): (&'static str, &'static str)) -> Failures {
    alloc::vec![(
        name,
        crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
            crate::verdicts::ConformanceError::Diverged { expected }
        ),
    )]
}

/// Expectation of the declared-dimension fixture.
const DECLARED_COSTS: &str =
    "the adapter declares every abstract cost in its own dimension with its exact units";

#[test]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A gate that compares the evidence's event and heads but never branch names or generations.
#[test]
fn a_head_only_evidence_gate_fails_both_gate_matrices() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::HeadOnlyEvidenceGate])?,
        both_gate_matrices("evidence from another generation executed the published selection")
    );
    Ok(())
}

/// A gate that delegates with the published decision whenever the presented one shares its
/// selection identity, candidate, operation, and ordinal (so another campaign, state, parent
/// moment, guidance, profile, ticket, cost, or fence field executes).
#[test]
fn an_execution_fields_gate_fails_both_gate_matrices() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ExecutionFieldsGate])?,
        both_gate_matrices("a decision tampered in its campaign executed")
    );
    Ok(())
}

/// A gate that accepts any self-consistent durable transition from the planned branch value: a
/// compare-and-set loser reporting the winner's committed transition.
#[test]
fn a_self_consistent_evidence_gate_fails_both_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::SelfConsistentEvidenceGate])?,
        both_gate_matrices("a compare-and-set loser executed under a committed transition")
    );
    Ok(())
}

/// A cache check that falls back to replaying the presented payloads on payload, envelope,
/// parent, history, and Campaign denials accepts a rewritten payload that replays.
#[test]
fn a_cache_fallback_on_unpresented_denials_fails_the_cache_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CacheFallbackOnUnpresentedDenials])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![(
                "caches.validation_matrix",
                crate::verdicts::FixtureVerdict::NegativeFixturePassed {
                    forbidden: "a cache of a rewritten control payload that replays was used as \
                            current state",
                },
            )],
            "a cache of a rewritten control payload that replays was used as current state"
        )
    );
    Ok(())
}

/// The state-coverage fixture's verdict for a cache check that refuses every held validation: it
/// refuses the cache of a held state's lineage on an older graph with an undeclared denial.
fn stale_graph_cache_denial() -> crate::verdicts::FixtureVerdict {
    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
        crate::verdicts::ConformanceError::UndeclaredDenial {
            forbidden: "a cache of a held state's lineage on an older graph was used as current \
                        state",
            denial: alloc::boxed::Box::new(crate::verdicts::ConformanceError::Adapter(
                campaign_choregraph::AdapterError::NonFastForwardBranch,
            )),
        },
    )
}

/// A cache check that refuses every held validation Campaign accepts refuses the fresh cache
/// presented through a held projection older than the head.
#[test]
fn refusing_every_held_cache_fails_the_cache_matrix() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::RefuseHeldCaches])?,
        crate::tests::with_state_coverage(
            alloc::vec![(
                "caches.validation_matrix",
                crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                    crate::verdicts::ConformanceError::Diverged {
                        expected: "the adapter's shell accepts exactly what Campaign accepts",
                    }
                ),
            )],
            stale_graph_cache_denial(),
        )
    );
    Ok(())
}

/// A shell that serves its held projection on metadata, envelope, schema, projection-identity,
/// non-stale Campaign, and non-ambiguity history denials loads a merged control event along a
/// parent choice that names neither of its parents (the first such history denial the catalog
/// presents).
#[test]
fn a_journal_on_residual_load_denials_fails_the_load_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::JournalOnResidualLoadDenials])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![(
                "history.load_matrix",
                crate::verdicts::FixtureVerdict::NegativeFixturePassed {
                    forbidden: "a merged control event was loaded along a parent choice naming \
                            neither parent",
                },
            )],
            "a merged control event was loaded along a parent choice naming neither parent"
        )
    );
    Ok(())
}

/// Declaring a second dimension but charging every cost to the first.
#[test]
fn collapsed_cost_dimensions_fail_the_declared_dimensions_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::CollapsedCostDimensions])?,
        diverged(("budgets.declared_dimensions", DECLARED_COSTS))
    );
    Ok(())
}

/// Saturating cost units at `u8::MAX`.
#[test]
fn u8_saturated_units_fail_the_declared_dimensions_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::U8Units])?,
        diverged(("budgets.declared_dimensions", DECLARED_COSTS))
    );
    Ok(())
}

/// Writing one index draw regardless of the scenario: every ticket-issuing fixture whose draw
/// differs from the constant fails at the ticket's decoding.
#[test]
fn a_constant_index_draw_fails_the_ticket_decoding() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ConstantIndexDraw])?,
        super::undecoded(&super::ticket_fixtures_except(&["entropy.explore"]))
    );
    Ok(())
}

/// Replacing the second guidance level by the first ties the second-level family and the
/// middle-level family.
#[test]
fn first_level_only_guidance_fails_the_second_and_middle_level_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let mut expected = diverged((
        "selection.second_level",
        "the second level decides between candidates whose first levels tie",
    ));
    expected.extend(diverged((
        "selection.middle_level",
        "a middle level decides between candidates whose first levels tie",
    )));
    assert_eq!(failures(&[wrapper::Hole::FirstLevelOnly])?, expected);
    Ok(())
}
