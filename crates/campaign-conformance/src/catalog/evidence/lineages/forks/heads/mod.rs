//! The head events that end a fork: a consumer stop, a guidance event (the fill's next
//! improvement, or an equal-rank coalesced duplicate once the guidance room is used), a
//! single-candidate batch, a ranked selection, a result without findings, and an unobserved
//! closure of the latest outstanding selection.

/// Handle of the operation a fork's candidate-batch head admits (above every fill operation).
pub(super) const HEAD_OPERATION: u32 = 200;

/// Name of the request a fork's selection head answers.
const FORK_SELECTION_REQUEST: &str = "fork-selection-request";

/// Kind of the event that ends a fork.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) enum HeadKind {
    /// A consumer stop.
    Stop,
    /// A guidance event.
    Guidance,
    /// A candidate batch.
    Batch,
    /// A published selection.
    Selection,
    /// A result that settles an outstanding selection.
    Result,
    /// An unobserved closure that settles an outstanding selection.
    Closure,
}

/// Every head kind.
pub(in crate::catalog::evidence) const HEAD_KINDS: [HeadKind; 6] = [
    HeadKind::Stop,
    HeadKind::Guidance,
    HeadKind::Batch,
    HeadKind::Selection,
    HeadKind::Result,
    HeadKind::Closure,
];

impl HeadKind {
    /// Whether the head settles an outstanding selection.
    pub(in crate::catalog::evidence) const fn settles(self) -> bool {
        matches!(self, Self::Result | Self::Closure)
    }

    /// Control events the head leaves room for beyond a stop head at its depth: the stop of the
    /// still-open campaign, and a selection's settlement; none over a stopped base, where only
    /// settlements of outstanding selections are admissible.
    pub(super) const fn reserve(self, is_stopped: bool) -> usize {
        if is_stopped || matches!(self, Self::Stop) {
            0
        } else if matches!(self, Self::Selection) {
            2
        } else {
            1
        }
    }
}

impl super::shaped::Head for HeadKind {
    fn settles(self) -> bool {
        Self::settles(self)
    }

    fn reserve(self, is_stopped: bool) -> usize {
        Self::reserve(self, is_stopped)
    }

    fn event(
        self,
        session: &mut crate::harness::Session<'_, '_>,
        base: &campaign_core::FrontierState,
        point: (
            &crate::harness::oracle::HistoryView,
            &campaign_choregraph::ValidatedControlProjection,
        ),
        step: usize,
    ) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
        event(session, base, point, step, self)
    }
}

/// The head event of `kind` admitted against the fork point `(history, view)`, `step` fill events
/// beyond the dropped head of a fork over `base`; `None` when the point's state admits no such
/// head (after a stop only settlements are admissible).
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the head.
// r[impl campaign.conformance.negative]
pub(in crate::catalog::evidence) fn event(
    session: &mut crate::harness::Session<'_, '_>,
    base: &campaign_core::FrontierState,
    (history, view): (
        &crate::harness::oracle::HistoryView,
        &campaign_choregraph::ValidatedControlProjection,
    ),
    step: usize,
    kind: HeadKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let state = view.state();
    debug_assert_eq!(history.payloads.len(), view.control_events().len());
    debug_assert!(HEAD_KINDS.contains(&kind));
    if state.stopped.is_some() && !kind.settles() {
        return Ok(None);
    }
    if kind.settles() {
        return settlement(session, (history, state), kind);
    }
    if matches!(kind, HeadKind::Selection) {
        return selection(session, view);
    }
    if matches!(kind, HeadKind::Batch) {
        return super::fill::batch(session, state, HEAD_OPERATION);
    }
    if matches!(kind, HeadKind::Guidance) {
        return match super::fill::improved(session, base, state, step)? {
            Some(improved) => Ok(Some(improved)),
            None => super::fill::coalesced(state),
        };
    }
    let reason = session.consumer_stop()?;
    Ok(Some(campaign_core::admit_stop(state, reason)?))
}

/// The ranked selection Campaign plans over the fork point, published with its profile; `None`
/// when the planner waits or stops.
///
/// # Errors
///
/// Returns the Campaign or adapter error of the plan.
pub(in crate::catalog::evidence) fn selection(
    session: &crate::harness::Session<'_, '_>,
    view: &campaign_choregraph::ValidatedControlProjection,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let state = view.state();
    let profile = session.selection_profile(false)?;
    let request = campaign_core::SelectionRequest {
        request_id: crate::harness::suite_id(
            campaign_core::IdKind::Request,
            FORK_SELECTION_REQUEST,
        )?,
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile: profile.clone(),
        entropy: None,
        expected_generation: view.binding().generation,
        expected_head_id: view.binding().head_id,
    };
    let plan = campaign_choregraph::plan_next_from_published_projection(
        view,
        campaign_choregraph::PublishedSelectionRequest { request },
    )?;
    debug_assert_eq!(plan.state_id, state.state_id);
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Ok(None);
    };
    debug_assert_eq!(selection.state_before_id, state.state_id);
    Ok(Some(
        campaign_core::CampaignControlEvent::SelectionPublished(
            campaign_core::SelectionEvent::new(*selection, profile, None)?,
        ),
    ))
}

/// The latest published selection of `history` that `state` still holds outstanding.
pub(super) fn latest_outstanding<'h>(
    history: &'h crate::harness::oracle::HistoryView,
    state: &campaign_core::FrontierState,
) -> Option<&'h campaign_core::SelectionDecision> {
    let is_outstanding = |selection_id: campaign_core::Id| {
        state
            .outstanding
            .iter()
            .any(|held| held.selection_id == selection_id)
    };
    let latest = history
        .payloads
        .iter()
        .rev()
        .filter_map(|payload| {
            if let campaign_core::CampaignControlEvent::SelectionPublished(published) =
                &payload.control_event
            {
                Some(&published.decision)
            } else {
                None
            }
        })
        .find(|decision| is_outstanding(decision.selection_id));
    debug_assert!(latest.is_some() || state.outstanding.is_empty() || history.payloads.is_empty());
    latest
}

/// The adapter's result without findings or its unobserved closure of the latest published
/// selection of `history` that `state` still holds outstanding, admitted against `state`; `None`
/// when nothing is outstanding.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the settlement.
pub(in crate::catalog::evidence) fn settlement(
    session: &mut crate::harness::Session<'_, '_>,
    (history, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
    kind: HeadKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let Some(selection) = latest_outstanding(history, state) else {
        return Ok(None);
    };
    debug_assert!(kind.settles());
    if matches!(kind, HeadKind::Closure) {
        let closure = session.closure(selection)?;
        return Ok(Some(campaign_core::close_unobserved(state, closure)?));
    }
    let context = crate::port::ResultContext {
        campaign_id: state.admission.campaign_id,
        selection,
        children: &[],
        state,
    };
    let observation = session.harness.adapter.result(
        &crate::vocabulary::ScenarioOutcome {
            children: alloc::vec::Vec::new(),
            findings: 0,
        },
        &context,
    )?;
    debug_assert_eq!(latest_outstanding(history, state), Some(selection));
    Ok(Some(campaign_core::admit_result(state, observation)?))
}
