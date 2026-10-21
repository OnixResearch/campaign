//! Promotion claims on receipts of every kind under every combination of consumer signals.

pub(super) mod duplicates;
mod kinds;
pub(super) mod planned;
pub(super) mod stops;
pub(super) mod witness;

/// Number of consumer signals and of their combinations.
const SIGNAL_COUNT: u32 = 4;
const SIGNAL_COMBINATIONS: u32 = 1 << SIGNAL_COUNT;

/// Signal bits of one combination.
const TOP_RANKED: u32 = 1;
const SELECTED: u32 = 1 << 1;
const ZERO_EXIT: u32 = 1 << 2;
const CHECK_REFERENCE: u32 = 1 << 3;

/// Retention bound that makes the declared retention plan prune one standard candidate.
const RETAINED: u64 = 2;

/// Findings a reported result carries beside its child moment.
const REPORTED_FINDINGS: u32 = 1;

/// Draw that takes the explore branch at a non-zero exploration position.
pub(super) const EXPLORE: crate::vocabulary::ScenarioDraw = crate::vocabulary::ScenarioDraw {
    explore_draw: 0,
    index_draw: 1,
};

/// The consumer signals of one combination.
const fn signals(bits: u32) -> crate::port::AuthoritySignals {
    crate::port::AuthoritySignals {
        is_top_ranked: bits & TOP_RANKED != 0,
        is_selected: bits & SELECTED != 0,
        has_zero_exit_status: bits & ZERO_EXIT != 0,
        has_check_reference: bits & CHECK_REFERENCE != 0,
    }
}

/// Asks the adapter for the claims of every receipt under every combination of the four consumer
/// signals, from none to all; every claim must stay false and every receipt must validate.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` naming the receipt kind of the first claimed receipt and the typed
/// error of a receipt that does not validate.
pub(super) fn require_unclaimed(
    session: &mut crate::harness::Session<'_, '_>,
    receipts: &[(&'static str, campaign_core::CampaignReceipt)],
) -> crate::verdicts::ConformanceResult<()> {
    for (forbidden, receipt) in receipts {
        for bits in 0..SIGNAL_COMBINATIONS {
            let claims = session
                .harness
                .adapter
                .promotion_claims(receipt, &signals(bits));
            if claims != campaign_core::ReceiptClaims::no_claims() {
                return Err(crate::verdicts::ConformanceError::ForbiddenAccepted { forbidden });
            }
            campaign_core::validate_receipt(receipt, &claims)?;
        }
    }
    debug_assert!(signals(SIGNAL_COMBINATIONS.saturating_sub(1)).has_check_reference);
    debug_assert!(!signals(0).is_selected);
    Ok(())
}

/// Presents receipts of every non-selection kind (a refusal of an admitted candidate and of a
/// duplicate within one batch, and a wait and the recorded consumer stop each planned for a
/// ranked and for an exploratory request), each as soon as it is projected, and the event receipt
/// of every control event (a result reporting a child and a finding, a closure published after
/// the stop, a prune, and a retirement among them) against every state the session reaches,
/// under every signal combination; the adapter must attach no claim to any of them.
// r[impl campaign.receipts.non_claims]
// r[impl campaign.conformance.claims]
pub(super) fn non_selection_overclaim(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::super::common::AMPLE_BUDGET, |bounds| {
            bounds.max_outstanding = 1;
            bounds.max_retained_candidates = RETAINED;
        })?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Reject;
    witness::witnessed(&mut session)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let [refusal, same_batch] = duplicates::refusal_receipts(&mut session)?;
    require_unclaimed(
        &mut session,
        &[
            (
                "an admission refusal receipt carried a promotion claim",
                refusal,
            ),
            (
                "a same-batch duplicate refusal receipt carried a promotion claim",
                same_batch,
            ),
        ],
    )?;
    let selection = session.execute(None)?;
    let [wait, explored_wait] = decided(
        &mut session,
        |decision| matches!(decision, campaign_core::NextDecision::Wait(_)),
        "the outstanding bound makes the planner wait",
    )?;
    require_unclaimed(
        &mut session,
        &[
            ("a wait receipt carried a promotion claim", wait),
            (
                "an exploratory wait receipt carried a promotion claim",
                explored_wait,
            ),
        ],
    )?;
    report(&mut session, &selection)?;
    session.prune()?;
    retire_retained(&mut session)?;
    let explored = session.execute(Some(EXPLORE))?;
    let reason = session.consumer_stop()?;
    session.stop(reason)?;
    let [stopped, explored_stop] = decided(
        &mut session,
        |decision| matches!(decision, campaign_core::NextDecision::Stop(_)),
        "the stopped campaign plans its recorded stop",
    )?;
    require_unclaimed(
        &mut session,
        &[
            ("a stop receipt carried a promotion claim", stopped),
            (
                "an exploratory recorded-stop receipt carried a promotion claim",
                explored_stop,
            ),
        ],
    )?;
    session.close(&explored)?;
    super::super::common::require(
        session.state.stopped.is_some() && session.state.outstanding.is_empty(),
        "the closure after the stop drains the stopped campaign",
    )?;
    debug_assert!(!session.state.findings.is_empty());
    Ok(())
}

/// Settles `selection` by a result that reports one child moment and one finding.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the moment or the result.
fn report(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
) -> crate::verdicts::ConformanceResult<()> {
    session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    let reported = crate::vocabulary::ScenarioOutcome {
        children: alloc::vec![super::super::common::CHILD_ONE],
        findings: REPORTED_FINDINGS,
    };
    let accepted = session.settle(selection, &reported)?;
    super::super::common::require(
        accepted.children.len() == 1 && accepted.findings.len() == 1,
        "the result reports one child moment and one finding",
    )?;
    debug_assert_eq!(accepted.selection_id, selection.selection_id);
    Ok(())
}

/// Retires the first standard operation, in suite operation order, whose candidate the declared
/// retention plan kept: the choice never depends on the adapter's candidate identities.
fn retire_retained(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    for operation in super::super::common::STANDARD_OPERATIONS {
        let candidate = session.candidate_id(operation)?;
        if session.state.candidate(candidate)?.status == campaign_core::CandidateStatus::Retained {
            return session.retire(operation);
        }
    }
    Err(crate::verdicts::ConformanceError::Diverged {
        expected: "the retention plan kept a candidate",
    })
}

/// Receipt of one planned selection, wait, or stop.
///
/// # Errors
///
/// Returns the Campaign error of the projection.
pub(super) fn plan_receipt(
    session: &crate::harness::Session<'_, '_>,
    planned: &crate::harness::decisions::Planned,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    Ok(campaign_core::project_receipt(
        &session.state,
        &planned.plan,
        &planned.request,
        &planned.binding,
    )?)
}

/// Receipts of the ranked and of the exploratory plan of the current state, each required to
/// reach a decision `is_decided` accepts; the exploratory receipt binds its entropy ticket.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a plan, and `Diverged` when a plan reaches another
/// decision or the exploratory receipt binds no ticket.
fn decided(
    session: &mut crate::harness::Session<'_, '_>,
    is_decided: fn(&campaign_core::NextDecision) -> bool,
    expected: &'static str,
) -> crate::verdicts::ConformanceResult<[campaign_core::CampaignReceipt; 2]> {
    let ranked = session.plan(None)?;
    super::super::common::require(is_decided(&ranked.plan.decision), expected)?;
    let ranked = plan_receipt(session, &ranked)?;
    let explored = session.plan(Some(EXPLORE))?;
    super::super::common::require(is_decided(&explored.plan.decision), expected)?;
    let explored = plan_receipt(session, &explored)?;
    super::super::common::require(
        explored.entropy_ticket_id.is_some(),
        "the exploratory receipt binds its ticket",
    )?;
    debug_assert!(ranked.entropy_ticket_id.is_none());
    Ok([ranked, explored])
}
