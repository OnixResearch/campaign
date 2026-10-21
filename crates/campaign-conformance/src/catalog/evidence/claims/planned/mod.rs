//! Promotion claims on receipts of decisions the planner computes before any stop is recorded:
//! every planner stop reason (frontier, budget, selection limit, control-event limit, and the
//! policy finding limit) and the waits for results and for guidance, each planned for a ranked
//! request and for an exploratory request whose receipt binds its entropy ticket.

pub(super) mod limits;

/// Presents every planner-computed stop and wait receipt of a ranked and of an exploratory
/// request, each projected while the campaign is not stopped, under every signal combination, in
/// witnessed campaigns; the adapter must attach no claim to any of them.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` naming the first claimed receipt and `Diverged` when the planner
/// does not reach the expected decision.
// r[impl campaign.conformance.claims]
pub(in crate::catalog::evidence) fn planner_overclaim(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    frontier_and_guidance(harness)?;
    budget_and_results(harness)?;
    limits::selection_limit(harness)?;
    limits::control_event_limit(harness)?;
    limits::finding_limit(harness)
}

/// Receipt of a decision the planner computed on an unstopped campaign.
fn unstopped_receipt(
    session: &crate::harness::Session<'_, '_>,
    planned: &crate::harness::decisions::Planned,
    is_expected: bool,
    expected: &'static str,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    super::super::super::common::require(is_expected, expected)?;
    super::super::super::common::require(
        session.state.stopped.is_none(),
        "the receipt is projected before any stop is recorded",
    )?;
    debug_assert!(session.state.stopped.is_none());
    super::plan_receipt(session, planned)
}

/// Receipts of the ranked and the exploratory plan of the current unstopped state, each required
/// to reach the expected decision; the exploratory receipt binds its entropy ticket.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a plan, and `Diverged` when a plan does not reach
/// the expected decision or the exploratory receipt binds no ticket.
// r[impl campaign.receipts.non_claims]
// r[impl campaign.stop.non_claim]
pub(in crate::catalog::evidence::claims) fn unstopped_receipts(
    session: &mut crate::harness::Session<'_, '_>,
    is_expected: impl Fn(&crate::harness::decisions::Planned) -> bool,
    expected: &'static str,
) -> crate::verdicts::ConformanceResult<[campaign_core::CampaignReceipt; 2]> {
    let ranked = session.plan(None)?;
    let ranked = unstopped_receipt(session, &ranked, is_expected(&ranked), expected)?;
    let explored = session.plan(Some(super::EXPLORE))?;
    let explored = unstopped_receipt(session, &explored, is_expected(&explored), expected)?;
    super::super::super::common::require(
        explored.entropy_ticket_id.is_some() && ranked.entropy_ticket_id.is_none(),
        "the exploratory receipt binds its ticket and the ranked one none",
    )?;
    debug_assert!(explored.entropy_ticket_id.is_some());
    Ok([ranked, explored])
}

/// The ranked and exploratory receipts under their forbidden effects.
fn named(
    names: [&'static str; 2],
    receipts: [campaign_core::CampaignReceipt; 2],
) -> [(&'static str, campaign_core::CampaignReceipt); 2] {
    let [ranked_name, explored_name] = names;
    let [ranked, explored] = receipts;
    [(ranked_name, ranked), (explored_name, explored)]
}

/// An empty frontier stops as exhausted; an unguided candidate then waits for guidance.
fn frontier_and_guidance(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::super::common::ample(harness)?;
    super::witness::witnessed(&mut session)?;
    let exhausted = unstopped_receipts(
        &mut session,
        |planned| {
            crate::catalog::frontier::stop_reason(planned)
                == Some(campaign_core::StopReason::FrontierExhausted)
        },
        "an empty frontier plans the frontier-exhausted stop",
    )?;
    session.admit(&[super::super::super::common::operation(
        super::super::super::common::OP_A,
        crate::harness::history::ROOT,
        super::super::super::common::UNIT_COST,
    )])?;
    let unguided = unstopped_receipts(
        &mut session,
        |planned| {
            crate::catalog::frontier::wait_reason(planned)
                == Some(campaign_core::WaitReason::AwaitingGuidance)
        },
        "an unguided candidate plans a wait for guidance",
    )?;
    debug_assert_eq!(session.state.candidates.len(), 1);
    debug_assert!(session.state.stopped.is_none());
    let [ranked_exhausted, explored_exhausted] = named(
        [
            "a frontier-exhausted stop receipt carried a promotion claim",
            "an exploratory frontier-exhausted stop receipt carried a promotion claim",
        ],
        exhausted,
    );
    let [ranked_unguided, explored_unguided] = named(
        [
            "a wait-for-guidance receipt carried a promotion claim",
            "an exploratory wait-for-guidance receipt carried a promotion claim",
        ],
        unguided,
    );
    super::require_unclaimed(
        &mut session,
        &[
            ranked_exhausted,
            explored_exhausted,
            ranked_unguided,
            explored_unguided,
        ],
    )
}

/// One reservation exhausts the budget: the planner waits for its result, then stops.
fn budget_and_results(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::super::super::common::UNIT_COST, |_| {})?;
    super::witness::witnessed(&mut session)?;
    super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::common::UNIT_COST,
    )?;
    let selection = session.execute(None)?;
    let pending = unstopped_receipts(
        &mut session,
        |planned| {
            crate::catalog::frontier::wait_reason(planned)
                == Some(campaign_core::WaitReason::AwaitingResults)
        },
        "outstanding work plans a wait for results",
    )?;
    session.settle(&selection, &super::super::super::common::outcome(&[]))?;
    let exhausted = unstopped_receipts(
        &mut session,
        |planned| {
            matches!(
                crate::catalog::frontier::stop_reason(planned),
                Some(campaign_core::StopReason::BudgetExhausted { .. })
            )
        },
        "an exhausted budget plans the budget-exhausted stop",
    )?;
    debug_assert!(session.state.outstanding.is_empty());
    debug_assert!(session.state.stopped.is_none());
    let [ranked_pending, explored_pending] = named(
        [
            "a wait-for-results receipt carried a promotion claim",
            "an exploratory wait-for-results receipt carried a promotion claim",
        ],
        pending,
    );
    let [ranked_exhausted, explored_exhausted] = named(
        [
            "a budget-exhausted stop receipt carried a promotion claim",
            "an exploratory budget-exhausted stop receipt carried a promotion claim",
        ],
        exhausted,
    );
    super::require_unclaimed(
        &mut session,
        &[
            ranked_pending,
            explored_pending,
            ranked_exhausted,
            explored_exhausted,
        ],
    )
}
