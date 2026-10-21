//! Refused history shapes the history catalog presents: a renamed branch, a reset branch,
//! suffix events over a tip (a forged recorded fence, merged control events, Campaign-invalid
//! stale-state, post-stop, and replayed events, and forged envelopes), and same-depth and deeper
//! forks.

pub(super) mod envelopes;
pub(super) mod forks;
pub(super) mod suffixes;

/// Name of the renamed control branch.
const RENAMED_BRANCH: &str = "campaign-conformance-renamed";

/// Projection request of one presented history under its own binding.
pub(super) fn projection_request<'h>(
    session: &crate::harness::Session<'_, '_>,
    history: &'h crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlProjectionRequest<'h>> {
    Ok(campaign_choregraph::ControlProjectionRequest {
        graph: &history.graph,
        branch: &history.branch,
        parent_choices: &history.parent_choices,
        maximum_events: crate::harness::HISTORY_MAX_EVENTS,
        expected_binding: campaign_choregraph::derive_projection_binding(
            &history.graph,
            &history.branch,
            session.identities.projection_schema_id,
            session.identities.adapter_id,
            session.identities.control_domain_id,
        )?,
        payloads: &history.payloads,
    })
}

/// The history under a renamed control branch.
pub(super) fn renamed(
    honest: &crate::harness::oracle::HistoryView,
) -> crate::harness::oracle::HistoryView {
    crate::harness::oracle::HistoryView {
        branch: campaign_choregraph::BranchRef {
            name: alloc::string::String::from(RENAMED_BRANCH),
            ..honest.branch.clone()
        },
        ..honest.clone()
    }
}

/// The current graph with its branch reset onto the earlier head at a later generation.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` at the last generation.
pub(super) fn reset(
    earlier: &crate::harness::oracle::HistoryView,
    honest: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let reset = crate::harness::oracle::HistoryView {
        graph: honest.graph.clone(),
        branch: campaign_choregraph::BranchRef {
            generation: honest
                .branch
                .generation
                .checked_add(1)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
            ..earlier.branch.clone()
        },
        ..earlier.clone()
    };
    debug_assert!(reset.branch.generation > honest.branch.generation);
    Ok(reset)
}

/// `base` with `event` admitted as the head's control child, its draft edited by `forge`, and the
/// branch moved onto it one generation later.
fn extended_with(
    session: &crate::harness::Session<'_, '_>,
    base: &crate::harness::oracle::HistoryView,
    event: campaign_core::CampaignControlEvent,
    forge: impl FnOnce(&mut campaign_choregraph::EventDraft),
) -> crate::verdicts::ConformanceResult<(
    crate::harness::oracle::HistoryView,
    campaign_choregraph::EventId,
)> {
    let mut draft = campaign_choregraph::control_event_draft(
        &event,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
        Some(base.branch.head),
    )?;
    forge(&mut draft);
    let (graph, admitted) = admitted_or_held(&base.graph, draft)?;
    let mut payloads = base.payloads.clone();
    payloads.push(campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: admitted,
        control_event: event,
    });
    let extended = crate::harness::oracle::HistoryView {
        graph,
        branch: campaign_choregraph::BranchRef {
            head: admitted,
            generation: base
                .branch
                .generation
                .checked_add(1)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
            ..base.branch.clone()
        },
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads,
    };
    debug_assert_eq!(
        extended.payloads.len(),
        base.payloads.len().saturating_add(1)
    );
    Ok((extended, admitted))
}

/// `graph` with `draft` admitted, or unchanged when it already holds the identical event (a
/// writer's suffix over a held prefix may repeat the event the honest lineage published next).
///
/// # Errors
///
/// Returns the Choregraph error of any other refused admission.
pub(super) fn admitted_or_held(
    graph: &campaign_choregraph::HistoryGraph,
    draft: campaign_choregraph::EventDraft,
) -> crate::verdicts::ConformanceResult<(
    campaign_choregraph::HistoryGraph,
    campaign_choregraph::EventId,
)> {
    let identity = campaign_choregraph::HistoryGraph::expected_identity(&draft);
    match graph.admit(draft) {
        Ok(admitted) => Ok((admitted.graph, admitted.event)),
        Err(campaign_choregraph::HistoryError::NoChange) => Ok((graph.clone(), identity)),
        Err(error) => Err(error.into()),
    }
}

/// `base` with `event` admitted as the head's honest control child.
fn extended(
    session: &crate::harness::Session<'_, '_>,
    base: &crate::harness::oracle::HistoryView,
    event: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    Ok(extended_with(session, base, event, |_| {})?.0)
}
