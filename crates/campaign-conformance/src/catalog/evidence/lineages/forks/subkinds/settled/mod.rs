//! Heads Campaign plans or settles: the exploratory selection Campaign plans for the scenario's
//! exploring draw, a result of the latest outstanding selection reporting a child moment, a
//! finding, both, or neither, and the consumer, host-capacity, policy, and planner stops.

/// Name of the request a fork's exploratory selection head answers.
const EXPLORATION_REQUEST: &str = "fork-exploration-request";

/// Name of the request a fork's planner stop head answers.
const STOP_REQUEST: &str = "fork-stop-request";

/// Child moment a fork's result head reports (a moment no recipe or catalog history reports).
const FORK_CHILD: crate::vocabulary::MomentHandle = crate::vocabulary::MomentHandle(150);

/// Findings a finding-reporting result head carries.
const REPORTED_FINDINGS: u32 = 1;

/// Campaign's plan over the fork point `view` for a request under `profile` with `entropy`.
///
/// # Errors
///
/// Returns the Campaign or adapter error of the plan.
fn plan(
    view: &campaign_choregraph::ValidatedControlProjection,
    (name, profile, entropy): (
        &str,
        campaign_core::SelectionProfile,
        Option<campaign_core::EntropyTicket>,
    ),
) -> crate::verdicts::ConformanceResult<campaign_core::DecisionPlan> {
    let state = view.state();
    let request = campaign_core::SelectionRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, name)?,
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile,
        entropy,
        expected_generation: view.binding().generation,
        expected_head_id: view.binding().head_id,
    };
    let plan = campaign_choregraph::plan_next_from_published_projection(
        view,
        campaign_choregraph::PublishedSelectionRequest { request },
    )?;
    debug_assert_eq!(plan.state_id, state.state_id);
    Ok(plan)
}

/// The exploratory selection Campaign plans over the fork point for the exploring draw, with the
/// adapter's ticket for the point's state; `None` when the planner exploits, waits, or stops.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the ticket or the plan, and `Diverged` for a ticket
/// that does not carry the draw.
pub(super) fn exploratory(
    session: &mut crate::harness::Session<'_, '_>,
    view: &campaign_choregraph::ValidatedControlProjection,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let state = view.state();
    let profile = session.selection_profile(true)?;
    let draw = crate::catalog::evidence::claims::EXPLORE;
    let context = crate::port::EntropyContext {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        selection_profile_id: profile.profile_id,
        counter: state
            .selection_count
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
    };
    let ticket = crate::harness::decisions::entropy::decoded(
        session.harness.adapter.entropy_ticket(&draw, &context)?,
        &draw,
        &context,
    )?;
    let planned = plan(
        view,
        (EXPLORATION_REQUEST, profile.clone(), Some(ticket.clone())),
    )?;
    let campaign_core::NextDecision::Select(selection) = planned.decision else {
        return Ok(None);
    };
    if selection.entropy_id.is_none() {
        return Ok(None);
    }
    debug_assert_eq!(selection.state_before_id, state.state_id);
    Ok(Some(
        campaign_core::CampaignControlEvent::SelectionPublished(
            campaign_core::SelectionEvent::new(*selection, profile, Some(ticket))?,
        ),
    ))
}

/// The adapter's result of the latest outstanding selection with the child moment and findings
/// of `kind`, admitted against `state`; `None` when nothing is outstanding or the finding bound
/// admits no finding.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the child moment or the result.
pub(super) fn result(
    session: &mut crate::harness::Session<'_, '_>,
    (history, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
    kind: super::SubKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let Some(selection) = super::super::heads::latest_outstanding(history, state) else {
        return Ok(None);
    };
    let has_child = matches!(
        kind,
        super::SubKind::ChildResult | super::SubKind::ReportedResult
    );
    let findings = if matches!(
        kind,
        super::SubKind::FindingResult | super::SubKind::ReportedResult
    ) {
        REPORTED_FINDINGS
    } else {
        0
    };
    let has_finding_room = u64::try_from(state.findings.len())
        .ok()
        .and_then(|count| count.checked_add(u64::from(findings)))
        .is_some_and(|total| total <= state.admission.bounds.max_findings);
    if !has_finding_room {
        return Ok(None);
    }
    let mut children = alloc::vec::Vec::with_capacity(1);
    if has_child {
        if session.moment(FORK_CHILD).is_err() {
            session.add_moment(crate::catalog::common::child(FORK_CHILD))?;
        }
        children.push(FORK_CHILD);
    }
    let reserved = session.reserve_children(selection, &children)?;
    let context = crate::port::ResultContext {
        campaign_id: state.admission.campaign_id,
        selection,
        children: &reserved,
        state,
    };
    let observation = session.harness.adapter.result(
        &crate::vocabulary::ScenarioOutcome { children, findings },
        &context,
    )?;
    debug_assert_eq!(reserved.len(), usize::from(has_child));
    Ok(Some(campaign_core::admit_result(state, observation)?))
}

/// The stop head of `kind` admitted against the fork point: the adapter's consumer stop, the
/// host-capacity consumer stop, the policy stop under the finding-limit reason, or the stop
/// Campaign's ranked plan makes (`None` when the planner selects or waits).
///
/// # Errors
///
/// Returns the adapter or Campaign error of the stop or the plan.
pub(super) fn stop(
    session: &mut crate::harness::Session<'_, '_>,
    view: &campaign_choregraph::ValidatedControlProjection,
    kind: super::SubKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let state = view.state();
    let reason = match kind {
        super::SubKind::HostCapacityStop => campaign_core::StopReason::ConsumerRequested {
            reason_id: campaign_choregraph::control_capacity_reason_id(),
        },
        super::SubKind::PolicyStop => campaign_core::StopReason::PolicyRequested {
            reason_id: campaign_core::StoppingRule {
                finding_limit: Some(u64::from(REPORTED_FINDINGS)),
            }
            .policy_reason_id()?
            .ok_or(campaign_core::CampaignError::InvalidConfig)?,
        },
        super::SubKind::PlannerStop => {
            let planned = plan(
                view,
                (STOP_REQUEST, session.selection_profile(false)?, None),
            )?;
            let campaign_core::NextDecision::Stop(stop) = planned.decision else {
                return Ok(None);
            };
            stop.reason
        }
        _ => session.consumer_stop()?,
    };
    debug_assert!(
        state.stopped.is_none(),
        "a stop head follows a running point"
    );
    Ok(Some(campaign_core::admit_stop(state, reason)?))
}
