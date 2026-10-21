//! Rewritten, altered, and ambiguous history the adapter's load path must refuse, cold and
//! through the held projection.

/// Frame domain of suite-owned orphan history events.
const ORPHAN_DOMAIN: &[u8] = b"campaign-conformance-orphan";

/// Parent roles of the ambiguous merge head.
const LEFT_ROLE: &str = "campaign-left";
const RIGHT_ROLE: &str = "campaign-right";

pub(super) fn orphan_draft(payload: &[u8]) -> campaign_choregraph::EventDraft {
    campaign_choregraph::EventDraft::v1(
        campaign_choregraph::EventClass::Application,
        campaign_choregraph::Digest::derive(ORPHAN_DOMAIN, payload),
        alloc::vec::Vec::new(),
        campaign_choregraph::Digest::derive(ORPHAN_DOMAIN, b"schema"),
    )
}

/// Presents a history to the adapter's load path cold and through the session's held
/// projection; each answer must equal Campaign's, and each must be the denial `expected` names.
fn refuse_presented(
    session: &mut crate::harness::Session<'_, '_>,
    history: &crate::harness::oracle::HistoryView,
    expected_binding: &campaign_core::ProjectionBinding,
    forbidden: &'static str,
    expected: impl Fn(&crate::verdicts::ConformanceError) -> bool,
) -> crate::verdicts::ConformanceResult<()> {
    for presentation in crate::harness::oracle::PRESENTATIONS {
        let loaded = session.present_bound_history(
            history,
            expected_binding.clone(),
            presentation,
            forbidden,
        )?;
        session.harness.deny_with(
            loaded.map_err(crate::verdicts::ConformanceError::from),
            forbidden,
            &expected,
        )?;
    }
    Ok(())
}

pub(super) fn adapter_denial(
    expected: campaign_choregraph::AdapterError,
) -> impl Fn(&crate::verdicts::ConformanceError) -> bool {
    move |error| *error == crate::verdicts::ConformanceError::Adapter(expected.clone())
}

/// Opens a guided session and loads its honest history through the adapter first, so every
/// forged presentation follows an accepted load of the same campaign.
fn loaded_session<'s, 'a>(
    harness: &'s mut crate::harness::Driver<'a>,
) -> crate::verdicts::ConformanceResult<crate::harness::Session<'s, 'a>> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    session.load()?;
    Ok(session)
}

/// `honest` with its first control payload rewritten into its last.
///
/// # Errors
///
/// Returns `Diverged` for a history without two distinct payloads.
pub(super) fn rewritten(
    honest: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let mut rewritten = honest.payloads.clone();
    let replacement = rewritten
        .last()
        .map(|payload| payload.control_event.clone())
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the session published control events",
        })?;
    if let Some(first) = rewritten.first_mut() {
        first.control_event = replacement;
    }
    super::super::common::require(
        rewritten.len() == honest.payloads.len() && rewritten != honest.payloads,
        "the rewrite replaces one published payload",
    )?;
    debug_assert_eq!(rewritten.len(), honest.payloads.len());
    Ok(crate::harness::oracle::HistoryView {
        payloads: rewritten,
        ..honest.clone()
    })
}

/// `honest` with its last control payload rewritten into `replacement`, an event that replays on
/// the same earlier state, so the rewritten payloads replay to another state.
///
/// # Errors
///
/// Returns `Diverged` for a history without payloads or a replacement equal to the last payload.
pub(super) fn rewritten_tail(
    honest: &crate::harness::oracle::HistoryView,
    replacement: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let mut rewritten = honest.payloads.clone();
    let last = rewritten
        .last_mut()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the session published control events",
        })?;
    super::super::common::require(
        last.control_event != replacement,
        "the rewrite replaces the last published payload with another event",
    )?;
    last.control_event = replacement;
    debug_assert_eq!(rewritten.len(), honest.payloads.len());
    Ok(crate::harness::oracle::HistoryView {
        payloads: rewritten,
        ..honest.clone()
    })
}

pub(super) fn rewritten_payload(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = loaded_session(harness)?;
    let history = rewritten(&session.history())?;
    let binding = session.binding()?;
    refuse_presented(
        &mut session,
        &history,
        &binding,
        "a rewritten control payload was loaded as history",
        adapter_denial(campaign_choregraph::AdapterError::PayloadIdentityMismatch),
    )
}

/// `history` with an orphan event admitted after it was bound.
///
/// # Errors
///
/// Returns the Choregraph error of the admission.
pub(super) fn altered(
    history: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    Ok(crate::harness::oracle::HistoryView {
        graph: history.graph.admit(orphan_draft(b"altered"))?.graph,
        ..history.clone()
    })
}

pub(super) fn altered_graph(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = loaded_session(harness)?;
    let binding = session.binding()?;
    let history = altered(&session.history())?;
    refuse_presented(
        &mut session,
        &history,
        &binding,
        "history altered after binding was loaded as current",
        adapter_denial(campaign_choregraph::AdapterError::ProjectionBindingMismatch),
    )
}

/// The payloads of `history` under its branch moved onto a head that merges two orphan events.
///
/// # Errors
///
/// Returns the Choregraph error of the admissions.
pub(super) fn ambiguous(
    history: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let left = history.graph.admit(orphan_draft(b"left"))?;
    let right = left.graph.admit(orphan_draft(b"right"))?;
    let mut parents = alloc::vec![
        campaign_choregraph::ParentLink {
            role: alloc::string::String::from(LEFT_ROLE),
            event: left.event,
        },
        campaign_choregraph::ParentLink {
            role: alloc::string::String::from(RIGHT_ROLE),
            event: right.event,
        },
    ];
    parents.sort();
    let head = right.graph.admit(campaign_choregraph::EventDraft::v1(
        campaign_choregraph::EventClass::Application,
        campaign_choregraph::Digest::derive(ORPHAN_DOMAIN, b"ambiguous-head"),
        parents,
        campaign_choregraph::Digest::derive(ORPHAN_DOMAIN, b"schema"),
    ))?;
    debug_assert_ne!(left.event, right.event);
    debug_assert!(head.graph.contains(head.event));
    Ok(crate::harness::oracle::HistoryView {
        branch: campaign_choregraph::BranchRef {
            head: head.event,
            ..history.branch.clone()
        },
        graph: head.graph,
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads: history.payloads.clone(),
    })
}

pub(super) fn ambiguous_lineage(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = loaded_session(harness)?;
    let history = ambiguous(&session.history())?;
    let binding = campaign_choregraph::derive_projection_binding(
        &history.graph,
        &history.branch,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    refuse_presented(
        &mut session,
        &history,
        &binding,
        "an ambiguous control lineage was loaded as history",
        is_ambiguous,
    )
}

/// Whether a denial is Choregraph's ambiguous-lineage refusal.
pub(super) const fn is_ambiguous(error: &crate::verdicts::ConformanceError) -> bool {
    matches!(
        error,
        crate::verdicts::ConformanceError::Adapter(campaign_choregraph::AdapterError::History(
            campaign_choregraph::HistoryError::AmbiguousLineage { .. }
        ))
    )
}
