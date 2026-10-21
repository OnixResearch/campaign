//! Crossed-settlement fixtures: two executions of one candidate in flight settle in both orders,
//! and each result must settle exactly the executed selection with the children reserved for that
//! execution. A mapping that files an outcome under the candidate's oldest or newest in-flight
//! selection settles a selection that did not produce it in one of the two orders.

/// Forbidden effect of a result that settles another selection than the executed one.
const CROSSED: &str = "a result settled a selection other than the executed one";

/// Settles one execution with `children` and requires the result to settle exactly that
/// selection, with the children reserved for it, while `other` (the execution still in flight,
/// if any) stays outstanding.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` when the result settles another selection and `Diverged` when its
/// children are not the ones reserved for the execution.
fn settle_exactly(
    session: &mut crate::harness::Session<'_, '_>,
    executed: &campaign_core::SelectionDecision,
    other: Option<&campaign_core::SelectionDecision>,
    children: &[crate::vocabulary::MomentHandle],
) -> crate::verdicts::ConformanceResult<()> {
    let accepted = session.settle(executed, &super::super::common::outcome(children))?;
    let is_other_settled = other.is_some_and(|other| {
        !session
            .state
            .outstanding
            .iter()
            .any(|outstanding| outstanding.selection_id == other.selection_id)
    });
    super::super::common::forbid(
        is_other_settled || accepted.selection_id != executed.selection_id,
        CROSSED,
    )?;
    debug_assert_eq!(accepted.selection_id, executed.selection_id);
    super::children::require_bound(session, executed, &accepted, children)
}

/// Executes the best candidate twice and settles both executions, the newer first when
/// `is_newer_first`: each result must settle its own selection with its own reserved children.
/// The first result reports a child only when the newer execution finishes first; the second
/// result always reports one.
///
/// # Errors
///
/// Returns the error of the scenario, `ForbiddenAccepted` for a crossed settlement, and
/// `Diverged` for crossed children.
fn settle_in_flight(
    harness: &mut crate::harness::Driver<'_>,
    is_newer_first: bool,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let older = session.execute(None)?;
    let newer = session.execute(None)?;
    super::super::common::require(
        older.selection_id != newer.selection_id && older.candidate_id == newer.candidate_id,
        "two executions of the best candidate are distinct selections",
    )?;
    debug_assert!(older.ordinal < newer.ordinal);
    if is_newer_first {
        session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    }
    session.add_moment(super::super::common::child(super::super::common::CHILD_TWO))?;
    let (first, second, first_children) = if is_newer_first {
        (&newer, &older, &[super::super::common::CHILD_ONE][..])
    } else {
        (&older, &newer, &[][..])
    };
    settle_exactly(&mut session, first, Some(second), first_children)?;
    settle_exactly(
        &mut session,
        second,
        None,
        &[super::super::common::CHILD_TWO],
    )?;
    let is_settled = |selection: &campaign_core::SelectionDecision| {
        session
            .state
            .settled
            .iter()
            .any(|settled| settled.selection_id == selection.selection_id)
    };
    super::super::common::require(
        session.state.outstanding.is_empty() && is_settled(&older) && is_settled(&newer),
        "both executed selections are settled",
    )
}

/// The older of two in-flight executions of one candidate finishes first.
// r[impl campaign.conformance.bounds]
pub(super) fn older_first(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    settle_in_flight(harness, false)
}

/// The newer of two in-flight executions of one candidate finishes first.
// r[impl campaign.conformance.bounds]
pub(super) fn newer_first(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    settle_in_flight(harness, true)
}
