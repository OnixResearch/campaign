//! Suffix events a writer appends to one tip history: the tip's next admissible event (a
//! consumer stop while the campaign runs, a closure of its latest outstanding selection once it
//! is stopped), a selection whose recorded publication fence is forged, a merged event, an event
//! admitted against an earlier state, new work after a stop, and a replayed event.

/// Parent role of the foreign parent of a merged control event.
const FOREIGN_PARENT_ROLE: &str = "campaign-foreign-parent";

/// Levels of the strict guidance improvement a writer publishes as new work.
const IMPROVED_LEVELS: [u32; 2] = [40, 4];

/// The next control event Campaign admits against `state`, the state at the head of `history`:
/// a consumer stop while the campaign is not stopped, otherwise the closure of its latest
/// outstanding selection.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the event, and `Diverged` for a stopped state with
/// nothing outstanding.
pub(in crate::catalog::evidence) fn next_event(
    session: &mut crate::harness::Session<'_, '_>,
    (history, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
    if state.stopped.is_none() {
        let reason = session.consumer_stop()?;
        return Ok(campaign_core::admit_stop(state, reason)?);
    }
    super::forks::heads::settlement(
        session,
        (history, state),
        super::forks::heads::HeadKind::Closure,
    )?
    .ok_or(crate::verdicts::ConformanceError::Diverged {
        expected: "a stopped tip holds an outstanding selection",
    })
}

/// `tip` extended by the selection Campaign plans over it (`view`) whose recorded publication
/// fence names the admission's branch value instead of the head it is admitted on; `None` when
/// the planner does not select.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the construction.
pub(in crate::catalog::evidence) fn forged_fence(
    session: &mut crate::harness::Session<'_, '_>,
    tip: &crate::harness::oracle::HistoryView,
    view: &campaign_choregraph::ValidatedControlProjection,
) -> crate::verdicts::ConformanceResult<Option<crate::harness::oracle::HistoryView>> {
    let Some(campaign_core::CampaignControlEvent::SelectionPublished(honest)) =
        super::forks::heads::selection(session, view)?
    else {
        return Ok(None);
    };
    let admitted = &view.state().admission.projection;
    debug_assert_eq!(tip.payloads.len(), view.control_events().len());
    debug_assert!(view.state().stopped.is_none());
    let mut forged = honest.decision.clone();
    forged.publication_plan.control_branch_id = admitted.control_branch_id;
    forged.publication_plan.expected_head_id = admitted.head_id;
    forged.publication_plan.expected_generation = admitted.generation;
    let forged = forged.derive()?;
    super::super::super::common::require(
        forged.publication_plan != honest.decision.publication_plan,
        "the forged fence names another branch value than the head it is admitted on",
    )?;
    let event = campaign_core::CampaignControlEvent::SelectionPublished(
        campaign_core::SelectionEvent::new(forged, honest.profile.as_ref().clone(), None)?,
    );
    Ok(Some(super::extended(session, tip, event)?))
}

/// `tip` extended by `event` also naming a foreign parent: without parent choices, with its
/// control parent chosen, with the foreign parent chosen, and with a choice naming the campaign's
/// control root, which is neither parent.
///
/// # Errors
///
/// Returns the Choregraph error of the construction and `Diverged` for a tip without a root.
pub(in crate::catalog::evidence) fn two_parent(
    session: &crate::harness::Session<'_, '_>,
    tip: &crate::harness::oracle::HistoryView,
    event: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<[crate::harness::oracle::HistoryView; 4]> {
    let orphan = tip
        .graph
        .admit(super::super::forged::orphan_draft(b"merge-parent"))?;
    let with_orphan = crate::harness::oracle::HistoryView {
        graph: orphan.graph,
        ..tip.clone()
    };
    let foreign = campaign_choregraph::ParentLink {
        role: alloc::string::String::from(FOREIGN_PARENT_ROLE),
        event: orphan.event,
    };
    let (unchosen, head) = super::extended_with(session, &with_orphan, event, |draft| {
        draft.parents.push(foreign);
        draft.parents.sort();
    })?;
    let mut control_chosen = unchosen.clone();
    control_chosen.parent_choices.insert(head, tip.branch.head);
    let mut foreign_chosen = unchosen.clone();
    foreign_chosen.parent_choices.insert(head, orphan.event);
    let root = tip
        .payloads
        .first()
        .map(|payload| payload.history_event_id)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the tip publishes a control root",
        })?;
    let mut neither_chosen = unchosen.clone();
    neither_chosen.parent_choices.insert(head, root);
    debug_assert_eq!(control_chosen.branch.head, head);
    debug_assert_eq!(foreign_chosen.branch.head, head);
    Ok([unchosen, control_chosen, foreign_chosen, neither_chosen])
}

/// `tip` extended by its last published control event that carries no publication fence (not a
/// selection) published again as a new Choregraph event: valid history whose replayed event
/// Campaign refuses as a duplicate.
///
/// # Errors
///
/// Returns the Choregraph error of the construction and `Diverged` for a tip without a
/// non-selection event.
pub(in crate::catalog::evidence) fn replayed(
    session: &crate::harness::Session<'_, '_>,
    tip: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let last = tip
        .payloads
        .iter()
        .rev()
        .map(|payload| &payload.control_event)
        .find(|event| {
            !matches!(
                event,
                campaign_core::CampaignControlEvent::SelectionPublished(_)
            )
        })
        .cloned()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the tip published a non-selection control event",
        })?;
    super::extended(session, tip, last)
}

/// `tip` (whose state is `state`) extended by new work admitted against `unstopped`, the latest
/// state before any stop (`work`): after a consumer stop of a running tip, or directly after the
/// stop of a stopped tip. Valid history whose work Campaign refuses as work after an accepted
/// stop.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the construction.
pub(in crate::catalog::evidence) fn work_after_stop(
    session: &mut crate::harness::Session<'_, '_>,
    (tip, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
    unstopped: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let work = work(session, unstopped)?;
    let stopped = if state.stopped.is_some() {
        tip.clone()
    } else {
        let reason = session.consumer_stop()?;
        let stop = campaign_core::admit_stop(state, reason)?;
        super::extended(session, tip, stop)?
    };
    let after = super::extended(session, &stopped, work)?;
    debug_assert!(unstopped.stopped.is_none());
    debug_assert!(after.payloads.len() > tip.payloads.len());
    Ok(after)
}

/// Handle of the operation new work admits when the standard operation C cannot be improved.
const WORK_OPERATION: u32 = 220;

/// New work admitted against `unstopped`: a strict guidance improvement of the standard
/// operation C while `unstopped` retains C and has guidance-record room, otherwise a
/// single-candidate batch of a new operation.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the work.
pub(in crate::catalog::evidence) fn work(
    session: &mut crate::harness::Session<'_, '_>,
    unstopped: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
    debug_assert!(
        unstopped.stopped.is_none(),
        "new work is admitted against a running state"
    );
    debug_assert!(
        unstopped.guidance_record_count <= unstopped.admission.bounds.max_guidance_records,
        "a state never counts more guidance records than its bound"
    );
    let improved = super::super::super::common::OP_C;
    let is_retained = session
        .candidate_id(improved)
        .ok()
        .is_some_and(|candidate_id| {
            unstopped
                .candidate(candidate_id)
                .is_ok_and(|candidate| candidate.status == campaign_core::CandidateStatus::Retained)
        });
    let has_room =
        unstopped.guidance_record_count < unstopped.admission.bounds.max_guidance_records;
    if is_retained && has_room {
        let record = session.guidance_record(&super::super::super::common::preference(
            improved,
            super::super::super::common::SECOND_GENERATION,
            IMPROVED_LEVELS,
        ))?;
        return Ok(campaign_core::admit_guidance(unstopped, record)?);
    }
    super::forks::fill::batch(session, unstopped, WORK_OPERATION)?.ok_or(
        crate::verdicts::ConformanceError::Diverged {
            expected: "the latest state before the stop admits new work",
        },
    )
}

/// `tip` extended by `event`, an event Campaign admitted against an earlier state of the
/// campaign: valid history whose control event Campaign refuses.
///
/// # Errors
///
/// Returns the Choregraph error of the construction.
pub(in crate::catalog::evidence) fn stale(
    session: &crate::harness::Session<'_, '_>,
    tip: &crate::harness::oracle::HistoryView,
    event: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    super::extended(session, tip, event)
}
