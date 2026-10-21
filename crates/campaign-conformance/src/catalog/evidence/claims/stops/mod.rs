//! Promotion claims on receipts of stops before and after they are recorded. Every stop the
//! planner computes is planned, presented, recorded, and presented again: the frontier stop of an
//! empty frontier and of a frontier whose every candidate was selected, settled, and retired, the
//! budget stop, the selection-limit and control-event-limit stops, and the policy stop of a
//! reached finding limit. For each, the ranked and the exploratory plan receipt of the unrecorded
//! stop, then of the recorded stop, reach the claims oracle under every signal combination. The
//! consumer stop the adapter plans when its host graph leaves no room for another selection has no
//! plan receipt before it is recorded (`campaign.receipts.host_capacity`); it is recorded and its
//! plan receipts presented. Every session is witnessed, so the event receipt of each of its
//! control events (results reporting a child and a finding, closures, prunes, and retirements
//! among them) is presented against every state it reaches, the stopped state included. (The
//! recorded consumer stop is presented by `receipts.non_selection_overclaim`, unrecorded planner
//! stops and waits also by `receipts.planner_overclaim`.)

mod limits;

/// Forbidden effects of a claimed ranked and exploratory plan receipt of one stop.
type Names = [&'static str; 2];

/// Plans, records, and presents every planner stop reason and the host-capacity stop, each in
/// its own witnessed campaign; the adapter must attach no claim to any receipt.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` naming the first claimed receipt, the typed error of a receipt
/// that does not validate, and `Diverged` when the planner does not reach the expected stop.
// r[impl campaign.receipts.non_claims]
// r[impl campaign.stop.non_claim]
// r[impl campaign.conformance.claims]
pub(in crate::catalog::evidence) fn recorded_stop_overclaim(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    empty_frontier(harness)?;
    retired_frontier(harness)?;
    budget(harness)?;
    limits::selection_limit(harness)?;
    limits::control_event_limit(harness)?;
    limits::finding_limit(harness)?;
    limits::host_capacity(harness)
}

/// Presents the receipts of the stop the current unstopped state plans for a ranked and for an
/// exploratory request, required to satisfy `is_reason`, under every signal combination, and
/// returns its reason.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a plan or receipt, `Diverged` when a plan does not
/// stop for such a reason, and `ForbiddenAccepted` naming the first claimed receipt.
fn unrecorded(
    session: &mut crate::harness::Session<'_, '_>,
    is_reason: fn(&campaign_core::StopReason) -> bool,
    expected: &'static str,
    names: Names,
) -> crate::verdicts::ConformanceResult<campaign_core::StopReason> {
    let planned = session.plan(None)?;
    let reason = crate::catalog::frontier::stop_reason(&planned)
        .filter(is_reason)
        .ok_or(crate::verdicts::ConformanceError::Diverged { expected })?;
    let receipts = super::planned::unstopped_receipts(
        session,
        |planned| crate::catalog::frontier::stop_reason(planned).as_ref() == Some(&reason),
        expected,
    )?;
    debug_assert!(session.state.stopped.is_none());
    super::require_unclaimed(session, &named(names, receipts))?;
    Ok(reason)
}

/// Receipts under their forbidden effects.
fn named(
    names: Names,
    receipts: [campaign_core::CampaignReceipt; 2],
) -> [(&'static str, campaign_core::CampaignReceipt); 2] {
    let [ranked_name, explored_name] = names;
    let [ranked, explored] = receipts;
    [(ranked_name, ranked), (explored_name, explored)]
}

/// Records `reason` and presents the ranked and exploratory plan receipts of the recorded stop
/// under every signal combination (the witness presents the stop's event receipt).
///
/// # Errors
///
/// Returns the error of the stop, of a plan, or of a receipt, and `ForbiddenAccepted` naming the
/// first claimed receipt.
fn recorded(
    session: &mut crate::harness::Session<'_, '_>,
    reason: &campaign_core::StopReason,
    names: Names,
) -> crate::verdicts::ConformanceResult<()> {
    session.stop(reason.clone())?;
    let is_recorded = session.payloads.last().is_some_and(|payload| {
        matches!(
            &payload.control_event,
            campaign_core::CampaignControlEvent::StopAccepted(stop) if stop.stop.reason == *reason
        )
    });
    super::super::super::common::require(is_recorded, "the session published the recorded stop")?;
    let receipts = super::decided(
        session,
        |decision| matches!(decision, campaign_core::NextDecision::Stop(_)),
        "the stopped campaign plans its recorded stop",
    )?;
    debug_assert!(session.state.stopped.is_some());
    super::require_unclaimed(session, &named(names, receipts))
}

/// An empty frontier: the frontier-exhausted stop.
fn empty_frontier(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::super::common::ample(harness)?;
    super::witness::witnessed(&mut session)?;
    let reason = unrecorded(
        &mut session,
        |reason| *reason == campaign_core::StopReason::FrontierExhausted,
        "an empty frontier plans the frontier-exhausted stop",
        [
            "a frontier-exhausted stop receipt carried a promotion claim",
            "an exploratory frontier-exhausted stop receipt carried a promotion claim",
        ],
    )?;
    debug_assert!(session.state.candidates.is_empty());
    recorded(
        &mut session,
        &reason,
        [
            "a recorded frontier-exhausted stop receipt carried a promotion claim",
            "an exploratory recorded frontier-exhausted stop receipt carried a promotion claim",
        ],
    )
}

/// Every standard candidate selected and retired in ranked order, the first settled by a result
/// reporting a child and a finding, the second closed, the third settled: the frontier-exhausted
/// stop of a searched frontier.
fn retired_frontier(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::super::common::ample(harness)?;
    super::witness::witnessed(&mut session)?;
    super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::common::UNIT_COST,
    )?;
    for position in 0..super::super::super::common::STANDARD_OPERATIONS.len() {
        let selection = session.execute(None)?;
        match position {
            0 => super::report(&mut session, &selection)?,
            1 => session.close(&selection)?,
            _ => {
                session.settle(&selection, &super::super::super::common::outcome(&[]))?;
            }
        }
        let operation = session.operation_of(selection.candidate_id)?;
        session.retire(operation)?;
    }
    super::super::super::common::require(
        session.state.candidates.iter().all(|candidate| {
            matches!(
                candidate.status,
                campaign_core::CandidateStatus::Retired { .. }
            )
        }),
        "every standard candidate was selected and retired",
    )?;
    let reason = unrecorded(
        &mut session,
        |reason| *reason == campaign_core::StopReason::FrontierExhausted,
        "a frontier whose every candidate was retired plans the frontier-exhausted stop",
        [
            "a frontier-exhausted stop receipt of a searched frontier carried a promotion claim",
            "an exploratory frontier-exhausted stop receipt of a searched frontier carried a \
             promotion claim",
        ],
    )?;
    debug_assert_eq!(
        usize::try_from(session.state.selection_count).ok(),
        Some(super::super::super::common::STANDARD_OPERATIONS.len())
    );
    recorded(
        &mut session,
        &reason,
        [
            "a recorded frontier-exhausted stop receipt of a searched frontier carried a \
             promotion claim",
            "an exploratory recorded frontier-exhausted stop receipt of a searched frontier \
             carried a promotion claim",
        ],
    )
}

/// Budget for two standard selections.
const TWO_SELECTIONS: u64 = 2;

/// A pruned and a retired candidate, then a closed and a reported selection of the candidate
/// left exhaust a budget of two: the budget-exhausted stop.
fn budget(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(harness, TWO_SELECTIONS, |bounds| {
        bounds.max_retained_candidates = super::RETAINED;
    })?;
    super::witness::witnessed(&mut session)?;
    super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::common::UNIT_COST,
    )?;
    session.prune()?;
    super::retire_retained(&mut session)?;
    let closed = session.execute(None)?;
    session.close(&closed)?;
    let reported = session.execute(None)?;
    super::report(&mut session, &reported)?;
    let reason = unrecorded(
        &mut session,
        |reason| matches!(reason, campaign_core::StopReason::BudgetExhausted { .. }),
        "an exhausted budget plans the budget-exhausted stop",
        [
            "a budget-exhausted stop receipt carried a promotion claim",
            "an exploratory budget-exhausted stop receipt carried a promotion claim",
        ],
    )?;
    debug_assert!(session.state.outstanding.is_empty());
    recorded(
        &mut session,
        &reason,
        [
            "a recorded budget-exhausted stop receipt carried a promotion claim",
            "an exploratory recorded budget-exhausted stop receipt carried a promotion claim",
        ],
    )
}
