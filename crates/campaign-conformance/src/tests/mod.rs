//! Campaign-side conformance tests: the conforming pilots, broken wrappers, closeout-audit,
//! re-audit, third-audit input-class, fourth-audit generated-class, fifth-audit sampled-class,
//! sixth-audit extent-class, seventh-audit position-class, eighth-audit held-class, ninth-audit
//! holder-class, tenth-audit receipt-class, eleventh-audit pairing-class, and twelfth-review
//! forked-class loophole controls, reference-planner differential evidence, and authority.

mod authority;
mod broken;
mod consumers;
mod covering_classes;
mod differential;
mod extent_classes;
mod forked_classes;
mod generated_classes;
mod held_classes;
mod holder_classes;
mod input_classes;
mod loopholes;
mod pairing_classes;
mod pilots;
mod position_classes;
mod reaudit;
mod receipt_classes;
mod sampled_classes;

/// Suite profile taken from the embedded, freshness-checked default Campaign profile export.
pub(crate) fn default_profile() -> crate::verdicts::ConformanceResult<crate::inputs::SuiteProfile> {
    let profile = campaign_profile::default_profile().map_err(|_| {
        crate::verdicts::ConformanceError::InvalidProfile {
            requirement: "the embedded default profile loads",
        }
    })?;
    let campaign_profile::DeclaredSelectionMode::Exploratory {
        threshold_parts_per_million,
        exploration_order,
    } = profile.selection_mode
    else {
        return Err(crate::verdicts::ConformanceError::InvalidProfile {
            requirement: "the default profile declares exploratory selection",
        });
    };
    debug_assert!(threshold_parts_per_million <= campaign_core::ENTROPY_SCALE);
    debug_assert!(!exploration_order.is_empty() && !profile.retention_rank_order.is_empty());
    Ok(crate::inputs::SuiteProfile {
        bounds: profile.bounds,
        duplicate_candidate: profile.admission.duplicate_candidate,
        exploratory_threshold_parts_per_million: threshold_parts_per_million,
        exploration_order,
        retention_rank_order: profile.retention_rank_order,
    })
}

/// Names of the fixtures whose verdict is not `Passed`, with their verdicts.
pub(crate) fn failures(
    report: &crate::verdicts::ConformanceReport,
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> {
    report
        .failures()
        .into_iter()
        .map(|outcome| (outcome.name, outcome.verdict.clone()))
        .collect()
}

/// Verdicts of the named fixtures (in suite order) when a ticket mapping issues a ticket that
/// does not decode to the scenario draw: positive fixtures diverge and negative fixtures fail
/// to set up their forbidden input, both at the harness's ticket decoding.
pub(crate) fn undecoded(
    fixtures: &[(&'static str, crate::verdicts::FixturePolarity)],
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> {
    fixtures
        .iter()
        .map(|(name, polarity)| {
            let diverged = crate::verdicts::ConformanceError::Diverged {
                expected: crate::harness::decisions::entropy::EXACT_DRAWS,
            };
            let verdict = match polarity {
                crate::verdicts::FixturePolarity::Positive => {
                    crate::verdicts::FixtureVerdict::PositiveFixtureFailed(diverged)
                }
                crate::verdicts::FixturePolarity::Negative => {
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(diverged)
                }
            };
            (*name, verdict)
        })
        .collect()
}

/// Fixtures that issue a ticket, in suite order, with their polarities.
pub(crate) const TICKET_FIXTURES: [(&str, crate::verdicts::FixturePolarity); 13] = [
    (
        "entropy.exploit",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "entropy.explore",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "entropy.wide_draws",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "entropy.invalid_ticket",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "fencing.gate_matrix_exploratory",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "history.state_coverage",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "receipts.promotion_overclaim",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "receipts.non_selection_overclaim",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "receipts.planner_overclaim",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "receipts.recorded_stop_overclaim",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "roles.collapsed_identity",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "scheduling.declared_difference",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "oracle.differential_evidence",
        crate::verdicts::FixturePolarity::Positive,
    ),
];

/// The ticket-issuing fixtures except the named ones.
pub(crate) fn ticket_fixtures_except(
    kept_passing: &[&str],
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixturePolarity)> {
    TICKET_FIXTURES
        .iter()
        .filter(|(name, _)| !kept_passing.contains(name))
        .copied()
        .collect()
}

/// `expected` with `history.state_coverage` answering `verdict`, in suite order.
pub(crate) fn with_state_coverage(
    expected: impl IntoIterator<Item = (&'static str, crate::verdicts::FixtureVerdict)>,
    verdict: crate::verdicts::FixtureVerdict,
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> {
    let names = crate::catalog::fixture_names();
    let mut failed: alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> =
        expected.into_iter().collect();
    failed.push(("history.state_coverage", verdict));
    failed.sort_by_key(|(name, _)| names.iter().position(|known| known == name));
    failed
}

/// Forbidden effect `history.state_coverage` reports first for a shell that loads a fork of the
/// history before a held state's head.
pub(crate) const FORK_THROUGH_HELD: &str =
    "a fork of the history before a held state's head was loaded as a fast-forward";

/// `expected` with `history.state_coverage` letting `forbidden` through, in suite order.
pub(crate) fn state_coverage_let_through(
    expected: impl IntoIterator<Item = (&'static str, crate::verdicts::FixtureVerdict)>,
    forbidden: &'static str,
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> {
    with_state_coverage(
        expected,
        crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
    )
}
