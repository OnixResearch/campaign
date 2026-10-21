//! Overclaims on one non-selection receipt kind at a time.

/// The claim fixtures' declared denials for an overclaim on each non-selection receipt kind: the
/// recorded-history fixture, for waits also the planner-decision fixture, and for event receipts
/// (the admission's, presented first by every witnessed claims session) and stops also the
/// planner-decision and recorded-stop fixtures.
const RECEIPT_KIND_CLAIMS: [(super::wrapper::ReceiptKind, &[super::Declared]); 4] = [
    (
        super::wrapper::ReceiptKind::Event,
        &[
            (
                "history.state_coverage",
                "a campaign admission event receipt carried a promotion claim",
            ),
            super::EVENT_RECEIPT_CLAIM,
            super::PLANNER_EVENT_CLAIM,
            super::RECORDED_EVENT_CLAIM,
        ],
    ),
    (
        super::wrapper::ReceiptKind::Refusal,
        &[
            (
                "history.state_coverage",
                "a held state's admission refusal receipt carried a promotion claim",
            ),
            (
                "receipts.non_selection_overclaim",
                "an admission refusal receipt carried a promotion claim",
            ),
        ],
    ),
    (
        super::wrapper::ReceiptKind::Wait,
        &[
            (
                "history.state_coverage",
                "a held state's ranked plan receipt carried a promotion claim",
            ),
            (
                "receipts.non_selection_overclaim",
                "a wait receipt carried a promotion claim",
            ),
            (
                "receipts.planner_overclaim",
                "a wait-for-guidance receipt carried a promotion claim",
            ),
        ],
    ),
    (
        super::wrapper::ReceiptKind::Stop,
        &[
            (
                "history.state_coverage",
                "a planner stop's receipt planned before recording carried a promotion claim",
            ),
            (
                "receipts.non_selection_overclaim",
                "a stop receipt carried a promotion claim",
            ),
            (
                "receipts.planner_overclaim",
                "a frontier-exhausted stop receipt carried a promotion claim",
            ),
            (
                "receipts.recorded_stop_overclaim",
                "a frontier-exhausted stop receipt carried a promotion claim",
            ),
        ],
    ),
];

#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.claims]
fn an_overclaim_on_any_one_receipt_kind_fails_the_non_selection_claim_fixture()
-> crate::verdicts::ConformanceResult<()> {
    for (kind, declared) in RECEIPT_KIND_CLAIMS {
        let report = super::run(&[super::wrapper::Loophole::OverclaimOn(kind)])?;
        assert_eq!(
            super::super::failures(&report),
            super::let_through(declared)
        );
    }
    Ok(())
}
