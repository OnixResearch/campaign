//! Idempotent republication: a host that durably admitted a control event's draft but crashed
//! before its branch compare-and-set republishes the identical event to the same publication.

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

/// The history after the host durably admitted `publication`'s draft but not its branch update.
fn crashed(
    hosted: &crate::suite::configured::Hosted,
    publication: &campaign_choregraph::ControlPublication,
) -> crate::suite::configured::Hosted {
    let mut crashed = hosted.clone();
    crashed.graph = publication.graph_after_admission().clone();
    crashed
}

/// Requires `republished` to be `original` republished through `recovered`: the same draft,
/// event, graph, branch plan, and control event, bound to the recovered projection, whose binding
/// differs from the original only in the identity of the graph that now holds the draft.
fn assert_republished(
    republished: &campaign_choregraph::ControlPublication,
    original: &campaign_choregraph::ControlPublication,
    recovered: &campaign_choregraph::ValidatedControlProjection,
) {
    assert_eq!(republished.payload_id(), original.payload_id());
    assert_eq!(republished.draft(), original.draft());
    assert_eq!(republished.history_event_id(), original.history_event_id());
    assert_eq!(
        republished.graph_after_admission(),
        original.graph_after_admission()
    );
    assert_eq!(republished.branch_plan(), original.branch_plan());
    assert_eq!(republished.control_event(), original.control_event());
    assert_eq!(republished.binding(), recovered.binding());
    assert_eq!(
        *original.binding(),
        campaign_core::ProjectionBinding {
            projection_id: original.binding().projection_id,
            graph_id: original.binding().graph_id,
            ..recovered.binding().clone()
        }
    );
}

/// After a partial commit the planner refuses to plan past the pending selection draft;
/// `pending_control_draft` names it, and its republication is the original publication without
/// admitting the draft again; the gate grants it once the branch update commits.
#[test]
// r[verify campaign.publication_fence.republication]
// r[verify campaign.publication_fence.pending_recovery]
fn republishing_after_a_partial_commit_yields_the_same_publication() -> Result<(), String> {
    let hosted = guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let crashed = crashed(&hosted, &publication);
    let recovered = crate::suite::identity::observed(crashed.validate())?;
    assert_eq!(recovered.control_events(), held.control_events());
    assert_eq!(
        campaign_choregraph::plan_next_from_published_projection(
            &recovered,
            campaign_choregraph::PublishedSelectionRequest {
                request: crate::suite::selection::request_for(&recovered)?,
            },
        ),
        Err(campaign_choregraph::AdapterError::PendingControlDraft {
            event_id: publication.history_event_id()
        })
    );
    let pending = crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
        &recovered,
        &crate::suite::identity::observed(crashed.request())?,
    ))?
    .ok_or("the selection draft is pending")?;
    assert_eq!(
        pending.control_event_id(),
        selection.publication_plan.event_id
    );
    let republished = crate::suite::identity::observed(
        crashed.publish(&recovered, crate::suite::selection::event(&selection)),
    )?;
    assert_republished(&republished, &publication, &recovered);
    assert_eq!(republished.graph_after_admission(), &crashed.graph);
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &republished,
            &crate::suite::selection::exact_durable_evidence(&republished),
        ),
        Ok(true)
    );
    let committed = crashed.committed(&republished);
    let granted = crate::suite::identity::observed(committed.validate())?;
    assert_eq!(granted.state().outstanding.len(), 1);
    assert_eq!(
        committed.publish(&granted, crate::suite::selection::event(&selection)),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    Ok(())
}

/// Republication does not count the held draft twice: with the graph limit exactly the
/// selection, its settlement, and the stop, the republished selection still fits, where counting
/// its draft again would exceed the limit.
#[test]
// r[verify campaign.publication_fence.republication]
fn a_held_draft_at_the_graph_bound_is_not_counted_twice() -> Result<(), String> {
    let hosted = guided(7)?;
    assert_eq!(hosted.graph.len(), 4);
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let crashed = crashed(&hosted, &publication);
    let recovered = crate::suite::identity::observed(crashed.validate())?;
    let republished = crate::suite::identity::observed(
        crashed.publish(&recovered, crate::suite::selection::event(&selection)),
    )?;
    assert_republished(&republished, &publication, &recovered);
    Ok(())
}

/// Another event on a graph holding the pending draft is refused by name without admitting
/// anything; once the pending selection is republished and committed, the other event publishes
/// as the next graph event.
#[test]
// r[verify campaign.publication_fence.republication]
// r[verify campaign.publication_fence.pending]
fn another_event_is_refused_until_the_pending_draft_completes() -> Result<(), String> {
    let hosted = guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let crashed = crashed(&hosted, &publication);
    let recovered = crate::suite::identity::observed(crashed.validate())?;
    let stop = crate::suite::integrity::consumer_stop(recovered.state(), b"republication-stop")?;
    assert_eq!(
        crashed.publish(&recovered, stop),
        Err(campaign_choregraph::AdapterError::PendingControlDraft {
            event_id: publication.history_event_id()
        })
    );
    let republished = crate::suite::identity::observed(
        crashed.publish(&recovered, crate::suite::selection::event(&selection)),
    )?;
    let committed = crashed.committed(&republished);
    let granted = crate::suite::identity::observed(committed.advance(&recovered))?;
    let stop = crate::suite::integrity::consumer_stop(granted.state(), b"republication-stop")?;
    let stopped = crate::suite::identity::observed(committed.publish(&granted, stop))?;
    assert_eq!(
        stopped.graph_after_admission().len(),
        crashed
            .graph
            .len()
            .checked_add(1)
            .ok_or("graph length overflow")?
    );
    let stopped_view = crate::suite::identity::observed(committed.committed(&stopped).validate())?;
    assert_eq!(stopped_view.state().outstanding.len(), 1);
    assert!(stopped_view.state().stopped.is_some());
    Ok(())
}
