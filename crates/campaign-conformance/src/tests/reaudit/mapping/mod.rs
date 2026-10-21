//! Result, finding, cache, load, identity, rank, and ticket loopholes of the re-audit.

/// Divergence of the result fixtures when an accepted child is missing or cross-wired.
const UNBOUND_CHILD: crate::verdicts::ConformanceError =
    crate::verdicts::ConformanceError::Diverged {
        expected: "each accepted child binds its own reserved payload and reservation",
    };

/// H1: the second path of a shared discovery drops its rediscovered child.
#[test]
fn s_h1_deduplicated_rediscoveries_fail_the_shared_discovery_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::DedupRediscoveredChildren])?,
        alloc::vec![(
            "results.shared_discovery",
            crate::verdicts::FixtureVerdict::PositiveFixtureFailed(UNBOUND_CHILD),
        )]
    );
    Ok(())
}

/// H2: the children of a forked result swap payloads (every result with two or more children).
#[test]
fn s_h2_cross_wired_fork_payloads_fail_every_fork_fixture() -> crate::verdicts::ConformanceResult<()>
{
    let failed = super::failures(&[super::wrapper::Hole::CrossWiredForkPayloads])?;
    assert!(failed.iter().all(|(_, verdict)| !verdict.is_passed()));
    assert_eq!(
        failed,
        alloc::vec![
            (
                "reconstruction.incremental_equals_rebuild",
                crate::verdicts::FixtureVerdict::PositiveFixtureFailed(UNBOUND_CHILD),
            ),
            (
                "reconstruction.conflicting_history",
                crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNBOUND_CHILD),
            ),
            (
                "results.fork_children",
                crate::verdicts::FixtureVerdict::PositiveFixtureFailed(UNBOUND_CHILD),
            ),
            (
                "results.child_counts",
                crate::verdicts::FixtureVerdict::PositiveFixtureFailed(UNBOUND_CHILD),
            ),
        ]
    );
    Ok(())
}

/// Divergence of the claims fixtures whose reported result must carry one child and one finding.
const UNREPORTED: crate::verdicts::ConformanceError = crate::verdicts::ConformanceError::Diverged {
    expected: "the result reports one child moment and one finding",
};

/// Divergence of the role fixture whose second sampled result must carry one child and one
/// finding.
const UNREPORTED_ROLE: crate::verdicts::ConformanceError =
    crate::verdicts::ConformanceError::Diverged {
        expected: "the second sampled execution reports a child moment and a finding",
    };

/// C4 (dropping) and H3: every finding is dropped from the result; the claims fixtures and the
/// role fixture that settle a result reporting a finding diverge too.
#[test]
fn probe_c4_and_s_h3_dropped_findings_fail_the_findings_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let failed = super::failures(&[super::wrapper::Hole::DroppedFindings])?;
    assert_eq!(failed.len(), 6);
    assert_eq!(
        failed,
        crate::tests::with_state_coverage(
            alloc::vec![
                (
                    "results.findings",
                    crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
                        crate::verdicts::ConformanceError::Diverged {
                            expected: "the result reports every observed finding under its own identity",
                        }
                    ),
                ),
                (
                    "receipts.non_selection_overclaim",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED),
                ),
                (
                    "receipts.planner_overclaim",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                        crate::verdicts::ConformanceError::Diverged {
                            expected: "a reached finding limit plans the policy stop",
                        }
                    ),
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED),
                ),
                (
                    "roles.collapsed_identity",
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED_ROLE),
                ),
            ],
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "the canonical recipe reaches its declared class"
                }
            )
        )
    );
    Ok(())
}

/// C4 (fabricating): every result reports one finding that was never observed; the claims
/// fixtures and the role fixture that settle a result reporting one finding diverge too.
#[test]
fn probe_c4_fabricated_findings_fail_the_findings_fixture() -> crate::verdicts::ConformanceResult<()>
{
    let mut expected =
        super::positive_diverged(("results.findings", "a result without findings reports none"));
    expected.extend([
        (
            "receipts.non_selection_overclaim",
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED),
        ),
        (
            "receipts.recorded_stop_overclaim",
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED),
        ),
        (
            "roles.collapsed_identity",
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(UNREPORTED_ROLE),
        ),
    ]);
    assert_eq!(expected.len(), 4);
    assert_eq!(
        super::failures(&[super::wrapper::Hole::FabricatedFindings])?,
        crate::tests::with_state_coverage(
            expected,
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "the canonical recipe reaches its declared class",
                },
            ),
        )
    );
    Ok(())
}

/// X1: settling the older of two outstanding selections of one candidate settles the newer.
#[test]
fn s_x1_results_keyed_by_candidate_fail_the_crossed_selection_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::ResultKeyedByCandidate])?,
        super::let_through(&[(
            "results.crossed_selection",
            "a result settled a selection other than the executed one",
        )])
    );
    Ok(())
}

/// C3 and C3b: a cache check by fresh binding and state identity, ignoring lineage, validates a
/// cache of a rewound branch through the held projection (and a stale state against payloads
/// lagging the current graph).
#[test]
fn probe_c3_a_binding_and_state_cache_check_fails_the_cache_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::BindingStateCache])?,
        crate::tests::state_coverage_let_through(
            super::let_through(&[(
                "caches.validation_matrix",
                "a cache of a rewound control branch was used as current state",
            )]),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}

/// K2: a cache check by projection identity validates a stale state under the current binding.
#[test]
fn s_k2_a_projection_id_only_cache_check_fails_the_cache_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::ProjectionIdOnlyCache])?,
        crate::tests::state_coverage_let_through(
            super::let_through(&[(
                "caches.validation_matrix",
                "a stale state under the current binding was used as current state",
            )]),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}

/// L1: the load path serves its journal for lagging, rewound, forked, or renamed histories.
#[test]
fn s_l1_a_journal_on_unpresented_denials_fails_the_load_matrix()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::JournalOnUnpresentedDenials])?,
        crate::tests::state_coverage_let_through(
            super::let_through(&[(
                "history.load_matrix",
                "a rewound control branch was loaded as current history",
            )]),
            "a branch rewound behind a held state was loaded as current history"
        )
    );
    Ok(())
}

/// I1: identities that change between calls.
#[test]
fn s_i1_unstable_identities_fail_the_identity_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::UnstableIdentities])?,
        super::positive_diverged((
            "identities.stable",
            "the adapter's identities are equal on every call"
        ))
    );
    Ok(())
}

/// M1: levels saturated at one byte before conversion.
#[test]
fn s_m1_saturating_ranks_fail_the_level_boundary_fixture() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        super::failures(&[super::wrapper::Hole::SaturatingRanks])?,
        super::positive_diverged((
            "guidance.monotone_levels",
            "a higher level maps to a strictly higher rank at every level boundary"
        ))
    );
    Ok(())
}

/// E1: ticket draws encoded big-endian: every ticket-issuing fixture fails at the ticket's
/// decoding except the invalid-ticket fixture, whose zero draws encode alike in either order.
#[test]
fn s_e1_a_big_endian_ticket_fails_the_ticket_decoding() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::BigEndianTicket])?,
        super::super::undecoded(&super::super::ticket_fixtures_except(&[
            "entropy.invalid_ticket"
        ]))
    );
    Ok(())
}
