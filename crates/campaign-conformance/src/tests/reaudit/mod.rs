//! Closeout re-audit loophole adapters kept as negative controls: each loophole that once passed
//! every fixture (reproducers C1-C4 and the scout's C4-C6, E1, G2, G3, H1-H3, I1, K2, L1, M1,
//! R1, X1) now fails exactly the named fixtures that close it, with their declared verdicts, in
//! debug and release builds.

mod mapping;
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

/// A positive fixture that diverged from its declared expectation.
fn positive_diverged(declared: Declared) -> Failures {
    let (name, expected) = declared;
    alloc::vec![(
        name,
        crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
            crate::verdicts::ConformanceError::Diverged { expected }
        ),
    )]
}

#[test]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// C1 and C1b: an own gate comparing only the selection identity grants a decision tampered
/// under the published identity (first presented: its campaign), for ranked and exploratory
/// selections alike.
#[test]
fn probe_c1_an_id_only_gate_fails_both_gate_matrices() -> crate::verdicts::ConformanceResult<()> {
    let tampered = "a decision tampered in its campaign executed";
    assert_eq!(
        failures(&[wrapper::Hole::IdOnlyGate])?,
        let_through(&[
            ("fencing.gate_matrix_ranked", tampered),
            ("fencing.gate_matrix_exploratory", tampered),
        ])
    );
    Ok(())
}

/// G2: a gate that grants a publication presented without evidence.
#[test]
fn s_g2_an_evidence_optional_gate_fails_both_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    let without_evidence = "a planned selection executed without durability evidence";
    assert_eq!(
        failures(&[wrapper::Hole::EvidenceOptionalGate])?,
        let_through(&[
            ("fencing.gate_matrix_ranked", without_evidence),
            ("fencing.gate_matrix_exploratory", without_evidence),
        ])
    );
    Ok(())
}

/// G3: a gate that grants every exploratory selection, including a ranked decision tampered to
/// carry a ticket under its published identity.
#[test]
fn s_g3_an_exploratory_gate_bypass_fails_both_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ExploratoryGateBypass])?,
        let_through(&[
            (
                "fencing.gate_matrix_ranked",
                "a decision tampered in its ticket executed",
            ),
            (
                "fencing.gate_matrix_exploratory",
                "a selection executed without a publication or evidence",
            ),
        ])
    );
    Ok(())
}

/// C2: exhaustive-search and safe-deletion claims on quiet non-selection receipts, first met on
/// the admission receipt every witnessed claims fixture presents.
#[test]
fn probe_c2_a_quiet_stop_claim_fails_every_claim_fixture() -> crate::verdicts::ConformanceResult<()>
{
    let failed = failures(&[wrapper::Hole::QuietStopClaim])?;
    assert_eq!(failed.len(), 4);
    assert_eq!(
        failed,
        crate::tests::state_coverage_let_through(
            let_through(&[
                (
                    "receipts.non_selection_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
                (
                    "receipts.planner_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
            ]),
            "a campaign admission event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Scout C4: an exhaustive-search claim on every receipt that binds a ticket, including the
/// exploratory wait and stop receipts the claim fixtures present first.
#[test]
// r[verify campaign.conformance.negative]
// r[verify campaign.receipts.non_claims]
fn s_c4_an_exploratory_receipt_overclaim_fails_every_receipt_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let failed = failures(&[wrapper::Hole::ExploratoryReceiptOverclaim])?;
    assert_eq!(failed.len(), 5);
    assert_eq!(
        failed,
        crate::tests::state_coverage_let_through(
            let_through(&[
                (
                    "receipts.promotion_overclaim",
                    "an exploratory selection receipt carried a promotion claim",
                ),
                (
                    "receipts.non_selection_overclaim",
                    "an exploratory wait receipt carried a promotion claim",
                ),
                (
                    "receipts.planner_overclaim",
                    "an exploratory frontier-exhausted stop receipt carried a promotion claim",
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    "an exploratory frontier-exhausted stop receipt carried a promotion claim",
                ),
            ]),
            "a selection event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Scout C5: an operation-success claim on a zero exit status without a check reference.
#[test]
fn s_c5_a_mixed_signal_overclaim_fails_both_claim_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let failed = failures(&[wrapper::Hole::MixedSignalOverclaim])?;
    assert_eq!(failed.len(), 5);
    assert_eq!(
        failed,
        crate::tests::state_coverage_let_through(
            let_through(&[
                (
                    "receipts.promotion_overclaim",
                    "rank, selection, exit status, or a check reference became promotion authority",
                ),
                (
                    "receipts.non_selection_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
                (
                    "receipts.planner_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    "a campaign admission event receipt carried a promotion claim",
                ),
            ]),
            "a campaign admission event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// Scout C6: an operation-success claim on the receipts of loaded result events, which every
/// witnessed claims fixture presents.
#[test]
fn s_c6_a_result_event_overclaim_fails_every_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let failed = failures(&[wrapper::Hole::ResultEventOverclaim])?;
    assert_eq!(failed.len(), 4);
    assert_eq!(
        failed,
        crate::tests::state_coverage_let_through(
            let_through(&[
                (
                    "receipts.non_selection_overclaim",
                    "a result event receipt carried a promotion claim",
                ),
                (
                    "receipts.planner_overclaim",
                    "a result event receipt carried a promotion claim",
                ),
                (
                    "receipts.recorded_stop_overclaim",
                    "a result event receipt carried a promotion claim",
                ),
            ]),
            "a result event receipt carried a promotion claim"
        )
    );
    Ok(())
}

/// R1: role collapse on exploratory executions only.
#[test]
// r[verify campaign.fd.roles.collapsed]
fn s_r1_an_exploratory_role_collapse_fails_the_role_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::ExploratoryRoleCollapse])?,
        let_through(&[(
            "roles.collapsed_identity",
            "the adapter stored candidate, work item, and result as one record",
        )])
    );
    Ok(())
}
