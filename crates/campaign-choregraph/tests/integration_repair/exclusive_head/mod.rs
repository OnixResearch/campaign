//! One pending control draft per branch head: while the graph holds a Campaign control draft whose
//! parent is the presented branch head, planning and every other publication on that head are
//! refused, so a compare-and-set loser or a stale publisher never consumes the winning lineage's
//! reserve; the host recovers the pending draft and republishes it, counted once.

mod pipelined;
mod recovery;
mod reserve;
mod residual;
mod siblings;

/// A guided campaign with one candidate under a graph of `graph_events` total events.
fn guided(graph_events: u32) -> Result<crate::suite::configured::Hosted, String> {
    crate::suite::configured::campaign::guided(
        crate::suite::configured::campaign::admitted_under(
            crate::suite::configured::campaign::root::Limits {
                graph_events,
                ..crate::suite::configured::campaign::root::Limits::standard(
                    crate::suite::constants::ROOT_GENERATION,
                    crate::suite::constants::MAX_EVENTS,
                )
            },
        )?,
        1,
    )
}

fn adapter_plan(
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_choregraph::AdapterResult<campaign_core::NextDecision>, String> {
    Ok(campaign_choregraph::plan_next_from_published_projection(
        held,
        campaign_choregraph::PublishedSelectionRequest {
            request: crate::suite::selection::request_for(held)?,
        },
    )
    .map(|plan| plan.decision))
}

/// The history after the host durably admitted `publication`'s draft but not its branch update.
fn crashed(
    hosted: &crate::suite::configured::Hosted,
    publication: &campaign_choregraph::ControlPublication,
) -> crate::suite::configured::Hosted {
    let mut crashed = hosted.clone();
    crashed.graph = publication.graph_after_admission().clone();
    crashed
}

const fn pending(
    publication: &campaign_choregraph::ControlPublication,
) -> campaign_choregraph::AdapterError {
    campaign_choregraph::AdapterError::PendingControlDraft {
        event_id: publication.history_event_id(),
    }
}

/// The unobserved closure of `selection` on `held`'s state.
fn closure_of(
    held: &campaign_choregraph::ValidatedControlProjection,
    selection: &campaign_core::SelectionDecision,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut closure = campaign_core::UnobservedClosure {
        closure_id: campaign_core::Id::absent(),
        campaign_id: held.state().admission.campaign_id,
        selection_id: selection.selection_id,
        reason_id: crate::suite::identity::derive(
            campaign_core::IdKind::Reason,
            b"pending-closure",
        ),
        uncertainty_acknowledged: true,
    };
    closure.closure_id =
        crate::suite::identity::observed(campaign_core::derive_closure_id(&closure))?;
    crate::suite::identity::observed(campaign_core::close_unobserved(held.state(), closure))
}

/// Requires the settlements of the granted `selection` and the stop each to publish on `hosted`.
fn reserve_is_intact(
    hosted: &crate::suite::configured::Hosted,
    selection: &campaign_core::SelectionDecision,
) -> Result<(), String> {
    let held = crate::suite::identity::observed(hosted.validate())?;
    assert_eq!(held.state().outstanding.len(), 1);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        held.state(),
        crate::suite::selection::result_for(&held, selection),
    ))?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"pending-winner-stop")?;
    for event in [result, closure_of(&held, selection)?, stop] {
        assert_eq!(hosted.publish(&held, event).map(|_| ()), Ok(()));
    }
    Ok(())
}

/// At a graph bound of exactly a selection, its settlement, and the stop, a compare-and-set loser
/// publishing from the winner's held projection over the graph that holds the winner's draft is
/// refused before admitting anything, while the winner's identical republication still succeeds;
/// once the winner commits, its result, its closure, and the stop all still fit.
#[test]
// r[verify campaign.publication_fence.pending]
fn a_cas_loser_is_refused_once_the_winners_draft_exists() -> Result<(), String> {
    let hosted = guided(7)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let winner = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let racing = crashed(&hosted, &winner);
    let losing_stop = crate::suite::integrity::consumer_stop(held.state(), b"pending-losing-stop")?;
    assert_eq!(
        racing.publish(&held, losing_stop.clone()),
        Err(pending(&winner))
    );
    assert_eq!(
        adapter_plan(&held)?.map(|_| ()),
        Ok(()),
        "the held view has no pending draft"
    );
    let republished = crate::suite::identity::observed(
        racing.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    assert_eq!(republished.history_event_id(), winner.history_event_id());
    assert_eq!(republished.graph_after_admission(), &racing.graph);
    let committed = hosted.committed(&winner);
    let granted = crate::suite::identity::observed(committed.validate())?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &winner,
            &crate::suite::selection::exact_durable_evidence(&winner),
        ),
        Ok(true)
    );
    assert_eq!(
        adapter_plan(&granted)?,
        Ok(campaign_core::NextDecision::Wait(
            campaign_core::WaitReason::AwaitingResults
        ))
    );
    assert_eq!(
        committed.publish(&held, losing_stop).map(|_| ()),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::StaleState
        )),
        "the loser's stop was computed from the superseded state"
    );
    reserve_is_intact(&committed, &selection)
}

/// A worker holding the projection before a committed selection presents its stale branch value
/// over the current graph: the committed selection is a draft on that stale head, so the stale
/// worker's planner and publication are both refused, `pending_control_draft` names the
/// committed selection, and the current campaign's settlements and stop still fit the graph.
#[test]
// r[verify campaign.publication_fence.pending]
fn a_stale_publisher_is_refused_over_the_current_graph() -> Result<(), String> {
    let hosted = guided(7)?;
    let stale_view = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&stale_view)?;
    let (current, publication) = hosted
        .clone()
        .append(crate::suite::selection::event(&selection))?;
    let mut stale = hosted.clone();
    stale.graph = current.graph.clone();
    let stale_stop = crate::suite::integrity::consumer_stop(stale_view.state(), b"pending-stale")?;
    assert_eq!(
        stale.publish(&stale_view, stale_stop),
        Err(pending(&publication))
    );
    let reloaded = crate::suite::identity::observed(stale.validate())?;
    assert_eq!(
        adapter_plan(&reloaded)?.map(|_| ()),
        Err(pending(&publication))
    );
    let draft = crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
        &stale_view,
        &crate::suite::identity::observed(stale.request())?,
    ))?
    .ok_or("the committed selection is pending on the stale head")?;
    assert_eq!(draft.history_event_id(), publication.history_event_id());
    assert_eq!(
        draft.control_event_id(),
        crate::suite::selection::event(&selection).event_id()
    );
    reserve_is_intact(&current, &selection)
}

/// Other children of the branch head are not pending control drafts: an event under another
/// envelope's schema, adapter, and control domain leaves planning and publication unaffected,
/// and nothing is pending without a draft.
#[test]
// r[verify campaign.publication_fence.pending]
fn only_control_drafts_under_the_projection_envelope_are_pending() -> Result<(), String> {
    let hosted = guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &held,
            &crate::suite::identity::observed(hosted.request())?
        ),
        Ok(None)
    );
    let foreign = crate::suite::configured::Envelope::foreign();
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"pending-foreign")?;
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &stop,
        foreign.schema,
        foreign.adapter,
        foreign.domain,
        Some(hosted.branch.head),
    ))?;
    let mut shared = hosted.clone();
    shared.graph = crate::suite::identity::observed(hosted.graph.admit(draft))?.graph;
    let view = crate::suite::identity::observed(shared.validate())?;
    assert!(matches!(
        adapter_plan(&view)?,
        Ok(campaign_core::NextDecision::Select(_))
    ));
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &view,
            &crate::suite::identity::observed(shared.request())?
        ),
        Ok(None)
    );
    assert_eq!(shared.publish(&view, stop).map(|_| ()), Ok(()));
    Ok(())
}
