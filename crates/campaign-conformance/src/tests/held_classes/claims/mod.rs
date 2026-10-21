//! Held-class claim shells: ticketed wait or stop claims and same-batch refusal claims.

/// A shell that claims exhaustive search on a ticketed wait or stop receipt overclaims on the
/// exploratory wait of the non-selection fixture and the exploratory receipt of the empty
/// frontier's stop before it is recorded in the planner and the recorded-stop fixtures.
#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
fn a_ticketed_wait_or_stop_claim_fails_both_claim_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let failed = super::failures(&[super::wrapper::Hole::TicketedWaitOrStopClaim])?;
    assert_eq!(failed.len(), 4);
    assert_eq!(
        failed,
        crate::tests::state_coverage_let_through(
            alloc::vec![
                super::passed((
                    "receipts.non_selection_overclaim",
                    "an exploratory wait receipt carried a promotion claim"
                )),
                super::passed((
                    "receipts.planner_overclaim",
                    "an exploratory frontier-exhausted stop receipt carried a promotion claim"
                )),
                super::passed((
                    "receipts.recorded_stop_overclaim",
                    "an exploratory frontier-exhausted stop receipt carried a promotion claim"
                )),
            ],
            "a planner stop's receipt planned before recording carried a promotion claim"
        )
    );
    Ok(())
}

/// A shell that claims exhaustive search on a same-batch duplicate refusal overclaims in the
/// non-selection fixture.
#[test]
// r[verify campaign.receipts.non_claims]
fn a_same_batch_refusal_claim_fails_the_non_selection_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        super::failures(&[super::wrapper::Hole::SameBatchRefusalClaim])?,
        crate::tests::state_coverage_let_through(
            alloc::vec![super::passed((
                "receipts.non_selection_overclaim",
                "a same-batch duplicate refusal receipt carried a promotion claim"
            ))],
            "a held state's same-batch refusal receipt carried a promotion claim"
        )
    );
    Ok(())
}
