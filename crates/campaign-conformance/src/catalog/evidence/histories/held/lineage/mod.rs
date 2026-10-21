//! The held lineage the matrix presents through: the session's campaign continued past its stop,
//! the histories forks start from, and every prefix of the final lineage with the history before
//! its head.

/// Findings the first settling result of the continued campaign reports.
const SETTLED_FINDINGS: u32 = 1;

/// Control events the continuation publishes after H1: a result, a selection, its closure, two
/// more selections, a consumer stop while both are outstanding, and a result that settles one
/// of them after the stop (the other stays outstanding).
const CONTINUED_EVENTS: usize = 7;

/// Selections still outstanding when the continuation stops.
const OUTSTANDING_AT_STOP: usize = 2;

/// The histories forks of the held prefixes start from, in lineage order: the history before
/// every head from the candidate batch on, each captured before that head's publication (its
/// graph holds no control draft after it). Continues the session's campaign past its stop.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the continuation or of a projection.
pub(super) fn bases(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::super::super::lineages::forks::Base>>
{
    let captured = continued(session)?;
    let mut bases = alloc::vec::Vec::with_capacity(
        earlier
            .opening
            .len()
            .saturating_add(captured.len())
            .saturating_add(2),
    );
    for history in &earlier.opening {
        bases.push(base_of(session, history.clone())?);
    }
    bases.push(clone_base(&earlier.before));
    bases.push(clone_base(&earlier.head));
    for history in captured {
        bases.push(base_of(session, history)?);
    }
    debug_assert_eq!(bases.len().saturating_add(1), session.payloads.len());
    debug_assert!(session.state.stopped.is_some());
    Ok(bases)
}

/// One fork base: `history` with Campaign's cold projection of it and its state.
///
/// # Errors
///
/// Returns Campaign's refusal of the history.
fn base_of(
    session: &crate::harness::Session<'_, '_>,
    history: crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<super::super::super::lineages::forks::Base> {
    let view = campaign_choregraph::validate_control_projection(
        &super::super::super::lineages::projection_request(session, &history)?,
    )?;
    let state = view.state().clone();
    Ok(super::super::super::lineages::forks::Base {
        history,
        view,
        state,
    })
}

/// A copy of one fork base.
fn clone_base(
    base: &super::super::super::lineages::forks::Base,
) -> super::super::super::lineages::forks::Base {
    super::super::super::lineages::forks::Base {
        history: base.history.clone(),
        view: base.view.clone(),
        state: base.state.clone(),
    }
}

/// The latest published selection of the session that is still outstanding.
///
/// # Errors
///
/// Returns `Diverged` when nothing is outstanding.
pub(super) fn outstanding(
    session: &crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
    session
        .payloads
        .iter()
        .rev()
        .find_map(|payload| {
            if let campaign_core::CampaignControlEvent::SelectionPublished(published) =
                &payload.control_event
            {
                Some(published.decision.clone())
            } else {
                None
            }
        })
        .filter(|decision| {
            session
                .state
                .outstanding
                .iter()
                .any(|held| held.selection_id == decision.selection_id)
        })
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the held lineage continues from an outstanding selection",
        })
}

/// Settles the session's outstanding selection by a result with a finding, executes and closes a
/// second selection, executes two more, stops while both are outstanding, and settles the first
/// of them by a result after the stop; returns the history before each of those heads as it
/// stood before the head's publication.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a step, and `Diverged` when nothing is
/// outstanding.
fn continued(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<crate::harness::oracle::HistoryView>> {
    let mut captured = alloc::vec::Vec::with_capacity(CONTINUED_EVENTS);
    let first = outstanding(session)?;
    captured.push(session.history());
    session.settle(
        &first,
        &crate::vocabulary::ScenarioOutcome {
            children: alloc::vec::Vec::new(),
            findings: SETTLED_FINDINGS,
        },
    )?;
    captured.push(session.history());
    let closed = session.execute(None)?;
    captured.push(session.history());
    session.close(&closed)?;
    captured.push(session.history());
    let settled_after_stop = session.execute(None)?;
    captured.push(session.history());
    session.execute(None)?;
    captured.push(session.history());
    let reason = session.consumer_stop()?;
    session.stop(reason)?;
    super::super::super::super::common::require(
        session.state.outstanding.len() == OUTSTANDING_AT_STOP,
        "the held campaign stops while selections are outstanding",
    )?;
    captured.push(session.history());
    session.settle(
        &settled_after_stop,
        &super::super::super::super::common::outcome(&[]),
    )?;
    debug_assert!(session.state.stopped.is_some());
    debug_assert_eq!(captured.len(), CONTINUED_EVENTS);
    Ok(captured)
}

/// Every prefix of the session's lineage under its final graph, with Campaign's cold projection,
/// its kind, and (from the second prefix on) the history before its head from `bases`.
///
/// # Errors
///
/// Returns Campaign's refusal of a prefix and `Diverged` when the session's transitions do not
/// match its payloads or `bases` do not precede the prefixes.
// r[impl campaign.conformance.negative]
pub(super) fn holders(
    session: &crate::harness::Session<'_, '_>,
    bases: alloc::vec::Vec<super::super::super::lineages::forks::Base>,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::Holder>> {
    super::super::super::super::common::require(
        session.transitions.len() == session.payloads.len()
            && bases.len().saturating_add(1) == session.payloads.len(),
        "the session committed one branch transition per control event and one fork base per \
         prefix after the root",
    )?;
    let mut holders = alloc::vec::Vec::with_capacity(session.payloads.len());
    let mut before = core::iter::once(None).chain(bases.into_iter().map(Some));
    for (length, transition) in (1..).zip(&session.transitions) {
        let history = prefix(session, &transition.branch_after, length);
        let view = campaign_choregraph::validate_control_projection(
            &super::super::super::lineages::projection_request(session, &history)?,
        )?;
        let kind = kind_of(&history, view.state());
        let base = before.next().flatten();
        debug_assert!(
            base.as_ref()
                .is_none_or(|base| base.history.payloads.len().saturating_add(1) == length)
        );
        holders.push(super::Holder {
            history,
            view,
            kind,
            base,
        });
    }
    debug_assert_eq!(holders.len(), session.payloads.len());
    Ok(holders)
}

/// The kind of a held prefix: what its head leaves the campaign in.
pub(super) fn kind_of(
    history: &crate::harness::oracle::HistoryView,
    state: &campaign_core::FrontierState,
) -> super::HolderKind {
    match history
        .payloads
        .last()
        .map(|payload| &payload.control_event)
    {
        Some(campaign_core::CampaignControlEvent::StopAccepted(_)) => super::HolderKind::Stopped,
        Some(campaign_core::CampaignControlEvent::ResultObserved(_)) => {
            super::HolderKind::SettledByResult
        }
        Some(campaign_core::CampaignControlEvent::UnobservedClosed(_)) => {
            super::HolderKind::SettledByClosure
        }
        Some(_) | None if state.outstanding.is_empty() => super::HolderKind::Idle,
        Some(_) | None => super::HolderKind::Outstanding,
    }
}

/// The first `event_count` control events of the session's lineage under its final graph, with the
/// branch value `branch` they were published at.
fn prefix(
    session: &crate::harness::Session<'_, '_>,
    branch: &campaign_choregraph::BranchRef,
    event_count: usize,
) -> crate::harness::oracle::HistoryView {
    crate::harness::oracle::HistoryView {
        graph: session.graph.clone(),
        branch: branch.clone(),
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads: session
            .payloads
            .get(..event_count)
            .map(<[_]>::to_vec)
            .unwrap_or_default(),
    }
}
