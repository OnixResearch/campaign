//! Stops at declared limits (the selection bound, the control-event bound, and the policy finding
//! limit) and at the host's graph bound, each planned, recorded, and presented in its own
//! witnessed campaign.

/// Graph event bound of the host-capacity campaign: the root moment and the control root, the
/// standard batch and its three guidance events, one selection, the child moment its result
/// reports, and that result hold 9 events, and another selection would need its own event, its
/// settlement, and the stop (12); the recorded capacity stop makes 10.
const CAPACITY_GRAPH_EVENTS: u32 = 11;

/// Control-event bound of the control-event-limit campaign: the admission, the standard batch and
/// its three guidance events, one selection, and its result hold 7 events, and another selection
/// would need its own event, its settlement, and the stop (10).
const REPORTED_SELECTION_EVENTS: u64 = 9;

/// One selection, settled by a result reporting a child and a finding, reaches a selection bound
/// of one: the selection-limit stop.
pub(super) fn selection_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(
        harness,
        super::super::super::super::common::AMPLE_BUDGET,
        |bounds| {
            bounds.max_selections = 1;
            bounds.max_outstanding = 1;
        },
    )?;
    super::super::witness::witnessed(&mut session)?;
    super::super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::super::common::UNIT_COST,
    )?;
    let selection = session.execute(None)?;
    super::super::report(&mut session, &selection)?;
    let reason = super::unrecorded(
        &mut session,
        |reason| *reason == campaign_core::StopReason::SelectionLimitReached,
        "a settled selection bound plans the selection-limit stop",
        [
            "a selection-limit stop receipt carried a promotion claim",
            "an exploratory selection-limit stop receipt carried a promotion claim",
        ],
    )?;
    debug_assert_eq!(session.state.selection_count, 1);
    super::recorded(
        &mut session,
        &reason,
        [
            "a recorded selection-limit stop receipt carried a promotion claim",
            "an exploratory recorded selection-limit stop receipt carried a promotion claim",
        ],
    )
}

/// A control-event bound that leaves room for the standard frontier and one selection settled by
/// a result reporting a child and a finding, but not for another selection: the
/// control-event-limit stop.
pub(super) fn control_event_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(
        harness,
        super::super::super::super::common::AMPLE_BUDGET,
        |bounds| bounds.max_control_events = REPORTED_SELECTION_EVENTS,
    )?;
    super::super::witness::witnessed(&mut session)?;
    super::super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::super::common::UNIT_COST,
    )?;
    let selection = session.execute(None)?;
    super::super::report(&mut session, &selection)?;
    let reason = super::unrecorded(
        &mut session,
        |reason| *reason == campaign_core::StopReason::ControlEventLimitReached,
        "a control-event bound without headroom plans the control-event-limit stop",
        [
            "a control-event-limit stop receipt carried a promotion claim",
            "an exploratory control-event-limit stop receipt carried a promotion claim",
        ],
    )?;
    debug_assert!(session.state.control_event_count < REPORTED_SELECTION_EVENTS);
    super::recorded(
        &mut session,
        &reason,
        [
            "a recorded control-event-limit stop receipt carried a promotion claim",
            "an exploratory recorded control-event-limit stop receipt carried a promotion claim",
        ],
    )
}

/// A settled finding reaches a declared finding limit: the policy stop the planner requests.
pub(super) fn finding_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::super::super::common::ample(harness)?;
    super::super::witness::witnessed(&mut session)?;
    super::super::planned::limits::at_finding_limit(&mut session)?;
    let planned = super::super::planned::limits::policy_plan(&mut session, None)?;
    let reason = crate::catalog::frontier::stop_reason(&planned)
        .filter(|reason| matches!(reason, campaign_core::StopReason::PolicyRequested { .. }))
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "a reached finding limit plans the policy stop",
        })?;
    let receipts = [
        super::super::planned::limits::limited_receipt(&mut session, None)?,
        super::super::planned::limits::limited_receipt(&mut session, Some(super::super::EXPLORE))?,
    ];
    super::super::require_unclaimed(
        &mut session,
        &super::named(
            [
                "a finding-limit policy stop receipt carried a promotion claim",
                "an exploratory finding-limit policy stop receipt carried a promotion claim",
            ],
            receipts,
        ),
    )?;
    debug_assert!(!session.state.findings.is_empty());
    debug_assert!(session.state.outstanding.is_empty());
    super::recorded(
        &mut session,
        &reason,
        [
            "a recorded finding-limit policy stop receipt carried a promotion claim",
            "an exploratory recorded finding-limit policy stop receipt carried a promotion claim",
        ],
    )
}

/// A host graph bound that leaves no room for another selection once one selection is settled
/// by a result reporting a child and a finding: the consumer stop the adapter plans under its
/// host-capacity reason through the published projection.
pub(super) fn host_capacity(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open_hosted(
        harness,
        super::super::super::super::common::AMPLE_BUDGET,
        CAPACITY_GRAPH_EVENTS,
        |_| {},
    )?;
    super::super::witness::witnessed(&mut session)?;
    super::super::super::super::common::guided_frontier(
        &mut session,
        super::super::super::super::common::UNIT_COST,
    )?;
    let selection = session.execute(None)?;
    super::super::report(&mut session, &selection)?;
    let reason = capacity_stop(&mut session)?;
    debug_assert!(session.state.outstanding.is_empty());
    debug_assert!(session.graph.len() < CAPACITY_GRAPH_EVENTS);
    super::recorded(
        &mut session,
        &reason,
        [
            "a recorded host-capacity stop receipt carried a promotion claim",
            "an exploratory recorded host-capacity stop receipt carried a promotion claim",
        ],
    )
}

/// The host-capacity stop the adapter plans for a ranked and for an exploratory request through
/// the published projection (Campaign projects no plan receipt of it before it is recorded).
///
/// # Errors
///
/// Returns the adapter or Campaign error of a plan, and `Diverged` when a plan is not the
/// host-capacity stop.
fn capacity_stop(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<campaign_core::StopReason> {
    let reason = campaign_core::StopReason::ConsumerRequested {
        reason_id: campaign_choregraph::control_capacity_reason_id(),
    };
    let ranked = session.plan(None)?;
    let explored = session.plan(Some(super::super::EXPLORE))?;
    super::super::super::super::common::require(
        crate::catalog::frontier::stop_reason(&ranked).as_ref() == Some(&reason)
            && crate::catalog::frontier::stop_reason(&explored).as_ref() == Some(&reason),
        "a host graph without room for another selection plans the host-capacity stop",
    )?;
    debug_assert!(explored.request.entropy.is_some());
    Ok(reason)
}
