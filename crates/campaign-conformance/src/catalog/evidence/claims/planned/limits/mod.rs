//! Planner stops at declared limits (the selection bound, the control-event bound, and the
//! policy finding limit), each planned for a ranked and for an exploratory request.

/// Finding limit of the declared stopping rule that makes the planner request the policy stop.
const FINDING_LIMIT: u64 = 1;

/// Findings the settled result reports to reach the finding limit.
const REPORTED_FINDINGS: u32 = 1;

/// One settled selection reaches a selection bound of one.
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
    session.settle(
        &selection,
        &super::super::super::super::common::outcome(&[]),
    )?;
    let limited = super::unstopped_receipts(
        &mut session,
        |planned| {
            crate::catalog::frontier::stop_reason(planned)
                == Some(campaign_core::StopReason::SelectionLimitReached)
        },
        "a settled selection bound plans the selection-limit stop",
    )?;
    debug_assert_eq!(session.state.selection_count, 1);
    debug_assert!(session.state.outstanding.is_empty());
    super::super::require_unclaimed(
        &mut session,
        &super::named(
            [
                "a selection-limit stop receipt carried a promotion claim",
                "an exploratory selection-limit stop receipt carried a promotion claim",
            ],
            limited,
        ),
    )
}

/// A control-event bound that holds only the admission and the stop reserve.
pub(super) fn control_event_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(
        harness,
        super::super::super::super::common::AMPLE_BUDGET,
        |bounds| bounds.max_control_events = campaign_core::MIN_CONTROL_EVENTS,
    )?;
    super::super::witness::witnessed(&mut session)?;
    let limited = super::unstopped_receipts(
        &mut session,
        |planned| {
            crate::catalog::frontier::stop_reason(planned)
                == Some(campaign_core::StopReason::ControlEventLimitReached)
        },
        "a control-event bound without headroom plans the control-event-limit stop",
    )?;
    debug_assert_eq!(session.state.control_event_count, 1);
    debug_assert!(session.state.stopped.is_none());
    super::super::require_unclaimed(
        &mut session,
        &super::named(
            [
                "a control-event-limit stop receipt carried a promotion claim",
                "an exploratory control-event-limit stop receipt carried a promotion claim",
            ],
            limited,
        ),
    )
}

/// A settled finding reaches a declared finding limit: the planner requests the policy stop for
/// a ranked and for an exploratory request.
pub(super) fn finding_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::super::super::common::ample(harness)?;
    super::super::witness::witnessed(&mut session)?;
    at_finding_limit(&mut session)?;
    let ranked = limited_receipt(&mut session, None)?;
    let explored = limited_receipt(&mut session, Some(super::super::EXPLORE))?;
    debug_assert!(!session.state.findings.is_empty());
    debug_assert!(session.state.outstanding.is_empty());
    super::super::require_unclaimed(
        &mut session,
        &super::named(
            [
                "a finding-limit policy stop receipt carried a promotion claim",
                "an exploratory finding-limit policy stop receipt carried a promotion claim",
            ],
            [ranked, explored],
        ),
    )
}

/// Guides the standard frontier, executes one selection, and settles it by a result whose
/// findings reach the declared finding limit.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a step.
pub(in crate::catalog::evidence::claims) fn at_finding_limit(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    super::super::super::super::common::guided_frontier(
        session,
        super::super::super::super::common::UNIT_COST,
    )?;
    let selection = session.execute(None)?;
    let reported = crate::vocabulary::ScenarioOutcome {
        children: alloc::vec::Vec::new(),
        findings: REPORTED_FINDINGS,
    };
    session.settle(&selection, &reported)?;
    debug_assert!(session.state.outstanding.is_empty());
    Ok(())
}

/// Receipt of the policy stop Campaign plans for a request whose stopping rule declares the
/// finding limit, ranked or exploratory with a ticket issued for that profile.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the plan, and `Diverged` when the planner does not
/// request the policy stop.
// r[impl campaign.stop.non_claim]
pub(in crate::catalog::evidence::claims) fn limited_receipt(
    session: &mut crate::harness::Session<'_, '_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    let planned = policy_plan(session, draw)?;
    super::unstopped_receipt(
        session,
        &planned,
        matches!(
            crate::catalog::frontier::stop_reason(&planned),
            Some(campaign_core::StopReason::PolicyRequested { .. })
        ),
        "a reached finding limit plans the policy stop",
    )
}

/// The plan Campaign computes for a request whose stopping rule declares the finding limit,
/// ranked or exploratory with a ticket issued for that profile.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the plan.
pub(in crate::catalog::evidence::claims) fn policy_plan(
    session: &mut crate::harness::Session<'_, '_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<crate::harness::decisions::Planned> {
    let current = session.load()?;
    let mut profile = session.selection_profile(draw.is_some())?;
    profile.stopping.finding_limit = Some(FINDING_LIMIT);
    profile.profile_id = campaign_core::Id::absent();
    let profile = profile.derive()?;
    let entropy = match draw {
        Some(draw) => Some(session.ticket(&draw, &profile)?),
        None => None,
    };
    let request = campaign_core::SelectionRequest {
        profile,
        entropy,
        ..session.request(None, current.binding())?
    };
    let plan = campaign_choregraph::plan_next_from_published_projection(
        &current,
        campaign_choregraph::PublishedSelectionRequest {
            request: request.clone(),
        },
    )?;
    debug_assert_eq!(plan.state_id, session.state.state_id);
    debug_assert_eq!(request.entropy.is_some(), draw.is_some());
    Ok(crate::harness::decisions::Planned {
        request,
        plan,
        binding: current.binding().clone(),
    })
}
