//! The suffix classes of one tip: merges of its next event, a forged fence, a stale-state
//! event, new work after a stop, a replayed event, and forged envelopes. A finished tip (stopped
//! with nothing outstanding) admits no next event; its merges and forged envelopes carry the new
//! work of the latest state before its stop instead.

/// The merged, forged-fence, stale-state, post-stop, replayed, and forged-envelope suffixes.
pub(super) fn appended(
    session: &mut crate::harness::Session<'_, '_>,
    tip: &super::Tip<'_>,
    built: &mut super::Built,
) -> crate::verdicts::ConformanceResult<()> {
    let state = tip.view.state();
    let is_finished = state.stopped.is_some() && state.outstanding.is_empty();
    let event = if is_finished {
        super::super::super::lineages::suffixes::work(session, tip.unstopped)?
    } else {
        super::super::super::lineages::suffixes::next_event(session, (tip.history, state))?
    };
    two_parents(session, tip, built, event.clone())?;
    if let Some(fence) =
        super::super::super::lineages::suffixes::forged_fence(session, tip.history, tip.view)?
    {
        super::push(session, built, super::Class::ForgedFence, fence, None)?;
    }
    let earlier = replacement(session, tip)?;
    let is_settlement = matches!(
        earlier,
        campaign_core::CampaignControlEvent::ResultObserved(_)
            | campaign_core::CampaignControlEvent::UnobservedClosed(_)
    );
    let stopped = (state.stopped.is_some() && !is_settlement).then_some(
        super::super::Answer::Refused(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::CampaignStopped,
        )),
    );
    let behind = super::super::super::lineages::suffixes::stale(session, tip.history, earlier)?;
    super::push(session, built, super::Class::Stale, behind, stopped)?;
    let work = super::super::super::lineages::suffixes::work_after_stop(
        session,
        (tip.history, state),
        tip.unstopped,
    )?;
    super::push(session, built, super::Class::WorkAfterStop, work, None)?;
    let replayed = super::super::super::lineages::suffixes::replayed(session, tip.history)?;
    super::push(session, built, super::Class::Replayed, replayed, None)?;
    debug_assert!(built.len() >= 4);
    let [metadata, class, schema, projection] =
        super::super::super::lineages::envelopes::forged_suffixes(session, tip.history, &event)?;
    super::push(
        session,
        built,
        super::Class::ForeignMetadata,
        metadata,
        None,
    )?;
    super::push(session, built, super::Class::NonApplication, class, None)?;
    super::push(session, built, super::Class::ForeignSchema, schema, None)?;
    super::push(
        session,
        built,
        super::Class::ForeignProjection,
        projection,
        None,
    )
}

/// The two-parent merges of the tip's next event `event`: without a parent choice, with its
/// control parent chosen, with its foreign parent chosen, and with a choice naming neither.
fn two_parents(
    session: &crate::harness::Session<'_, '_>,
    tip: &super::Tip<'_>,
    built: &mut super::Built,
    event: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(!tip.history.payloads.is_empty());
    let [unchosen, control, foreign, neither] =
        super::super::super::lineages::suffixes::two_parent(session, tip.history, event)?;
    let root = tip
        .history
        .payloads
        .first()
        .map(|payload| payload.history_event_id)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the tip publishes a control root",
        })?;
    let invalid_choice = super::super::Answer::Refused(campaign_choregraph::AdapterError::History(
        campaign_choregraph::HistoryError::InvalidParentChoice {
            event: neither.branch.head,
            selected: root,
        },
    ));
    super::push(
        session,
        built,
        super::Class::TwoParentUnchosen,
        unchosen,
        None,
    )?;
    super::push(
        session,
        built,
        super::Class::TwoParentControl,
        control,
        None,
    )?;
    super::push(
        session,
        built,
        super::Class::TwoParentForeign,
        foreign,
        None,
    )?;
    super::push(
        session,
        built,
        super::Class::TwoParentNeither,
        neither,
        Some(invalid_choice),
    )?;
    Ok(())
}

/// An event admitted against the state before the tip's head that differs from the head's event
/// (it replays in place of the head, and is stale after it): the next event of that state, else
/// the closure of its latest outstanding selection, else the host-capacity consumer stop.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the event and `Diverged` when none differs.
pub(super) fn replacement(
    session: &mut crate::harness::Session<'_, '_>,
    tip: &super::Tip<'_>,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
    debug_assert!(tip.base.history.payloads.len() < tip.history.payloads.len());
    debug_assert!(tip.base.state.control_event_count < tip.view.state().control_event_count);
    let head = super::super::shapes::forked::head_event(tip.history)?;
    let base = (&tip.base.history, &tip.base.state);
    let next = super::super::super::lineages::suffixes::next_event(session, base)?;
    let closure = super::super::super::lineages::forks::heads::settlement(
        session,
        base,
        super::super::super::lineages::forks::heads::HeadKind::Closure,
    )?;
    let capacity = campaign_core::StopReason::ConsumerRequested {
        reason_id: campaign_choregraph::control_capacity_reason_id(),
    };
    let stop = campaign_core::admit_stop(&tip.base.state, capacity).ok();
    [Some(next), closure, stop]
        .into_iter()
        .flatten()
        .find(|event| *event != head)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the state before the tip's head admits an event other than the head",
        })
}
