//! Budget reservation and unobserved-closure fixtures.

mod reservations;

/// Budget and closure fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "budgets.reservation",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: reservation,
    },
    super::Fixture {
        name: "budgets.declared_dimensions",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: reservations::declared_dimensions,
    },
    super::Fixture {
        name: "budgets.overdraw",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: overdraw,
    },
    super::Fixture {
        name: "closures.unobserved",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: unobserved,
    },
    super::Fixture {
        name: "closures.unacknowledged",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: unacknowledged,
    },
    super::Fixture {
        name: "closures.out_of_order",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: out_of_order,
    },
];

/// Abstract units a reserving operation costs.
const RESERVED_UNITS: u64 = 2;

/// Budget that fits one reserving operation but not two.
const ONE_RESERVATION_BUDGET: u64 = 3;

fn reservation(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, RESERVED_UNITS)?;
    let selection = session.execute(None)?;
    super::common::require(
        super::outcomes::reserved_units(&session) == RESERVED_UNITS,
        "a selection reserves exactly its abstract cost",
    )?;
    session.settle(&selection, &super::common::outcome(&[]))?;
    super::common::require(
        super::outcomes::reserved_units(&session) == RESERVED_UNITS,
        "an external outcome refunds no reservation",
    )
}

fn overdraw(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(harness, ONE_RESERVATION_BUDGET, |_| {})?;
    super::common::guided_frontier(&mut session, RESERVED_UNITS)?;
    let selection = session.execute(None)?;
    super::common::require(
        session.state.outstanding.len() == 1
            && super::outcomes::reserved_units(&session) <= ONE_RESERVATION_BUDGET,
        "one reservation fits the budget",
    )?;
    debug_assert_eq!(session.state.outstanding.len(), 1);
    debug_assert!(super::outcomes::reserved_units(&session) <= ONE_RESERVATION_BUDGET);
    let pending = session.plan(None)?;
    super::common::forbid(
        pending.selection().is_ok(),
        "a selection overdrew the abstract budget",
    )?;
    session.settle(&selection, &super::common::outcome(&[]))?;
    let settled = session.plan(None)?;
    super::common::forbid(
        settled.selection().is_ok(),
        "a settled outcome refunded the reserved budget",
    )?;
    let is_budget_stop = matches!(
        &settled.plan.decision,
        campaign_core::NextDecision::Stop(stop)
            if matches!(stop.reason, campaign_core::StopReason::BudgetExhausted { .. })
    );
    super::common::require(is_budget_stop, "an exhausted budget stops the campaign")
}

fn unobserved(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let reserved = super::outcomes::reserved_units(&session);
    session.close(&selection)?;
    super::common::require(
        session.state.outstanding.is_empty(),
        "the closure settles the selection",
    )?;
    super::common::require(
        session.state.discoveries.is_empty(),
        "a closure infers no child",
    )?;
    super::common::require(
        super::outcomes::reserved_units(&session) == reserved,
        "a closure refunds nothing",
    )
}

fn unacknowledged(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let mut closure = session.closure(&selection)?;
    closure.uncertainty_acknowledged = false;
    closure.closure_id = campaign_core::derive_closure_id(&closure)?;
    let closed = campaign_core::close_unobserved(&session.state, closure);
    session.harness.deny(
        closed.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::MissingUncertaintyAcknowledgement,
        "an unacknowledged closure settled a selection",
    )
}

/// Closes one of two in-flight executions of one candidate: exactly the named selection is
/// closed, while `other` (the execution still in flight, if any) stays outstanding.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` when the closure closes another selection, and the error of the
/// closure.
// r[impl campaign.conformance.bounds]
fn close_exactly(
    session: &mut crate::harness::Session<'_, '_>,
    closed: &campaign_core::SelectionDecision,
    other: Option<&campaign_core::SelectionDecision>,
) -> crate::verdicts::ConformanceResult<()> {
    session.close(closed)?;
    let is_outstanding = |selection: &campaign_core::SelectionDecision| {
        session
            .state
            .outstanding
            .iter()
            .any(|outstanding| outstanding.selection_id == selection.selection_id)
    };
    super::common::forbid(
        is_outstanding(closed) || other.is_some_and(|other| !is_outstanding(other)),
        crate::harness::settlement::CROSSED_CLOSURE,
    )
}

/// Executes the best candidate twice and closes both executions, the newer first when
/// `is_newer_first`: each closure must close exactly the named selection.
///
/// # Errors
///
/// Returns the error of the scenario and `ForbiddenAccepted` for a crossed closure.
fn close_in_flight(
    harness: &mut crate::harness::Driver<'_>,
    is_newer_first: bool,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let older = session.execute(None)?;
    let newer = session.execute(None)?;
    super::common::require(
        older.selection_id != newer.selection_id && older.candidate_id == newer.candidate_id,
        "two executions of the best candidate are distinct selections",
    )?;
    let (first, second) = if is_newer_first {
        (&newer, &older)
    } else {
        (&older, &newer)
    };
    debug_assert!(older.ordinal < newer.ordinal);
    debug_assert_ne!(first.selection_id, second.selection_id);
    close_exactly(&mut session, first, Some(second))?;
    close_exactly(&mut session, second, None)?;
    super::common::require(
        session.state.outstanding.is_empty(),
        "both executed selections are closed",
    )
}

/// Two in-flight executions of one candidate closed older-first, then (in a fresh campaign)
/// newer-first: a closure mapping keyed by candidate, or by its oldest or newest in-flight
/// selection, closes a selection other than the named one in one of the two orders.
// r[impl campaign.conformance.bounds]
fn out_of_order(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    close_in_flight(harness, false)?;
    close_in_flight(harness, true)
}
