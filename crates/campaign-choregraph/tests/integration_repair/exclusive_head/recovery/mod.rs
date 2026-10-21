//! Recovering a pending draft: the planner and every other publication refuse it by name,
//! `pending_control_draft` decodes it against the held projection, and its republication counts
//! it once, also at the graph bound.

/// Requires `crashed` to hold exactly `publication`'s draft as the pending draft of `recovered`,
/// decoded to `control_event`'s identities.
fn assert_decoded(
    crashed: &crate::suite::configured::Hosted,
    recovered: &campaign_choregraph::ValidatedControlProjection,
    publication: &campaign_choregraph::ControlPublication,
    control_event: &campaign_core::CampaignControlEvent,
) -> Result<(), String> {
    let draft = crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
        recovered,
        &crate::suite::identity::observed(crashed.request())?,
    ))?
    .ok_or("the crashed publication is pending")?;
    assert_eq!(draft.history_event_id(), publication.history_event_id());
    assert_eq!(draft.control_event_id(), control_event.event_id());
    assert_eq!(draft.payload_id(), publication.payload_id());
    Ok(())
}

/// At a graph bound of five events the planner plans the host-capacity stop as the
/// graph's last event; after a partial commit of that stop the planner and any other stop are
/// refused with the pending draft, which `pending_control_draft` decodes, and republishing it
/// fits the bound (the held draft counts once) and records the stop.
#[test]
// r[verify campaign.publication_fence.pending_recovery]
fn a_held_capacity_stop_is_recovered_at_the_graph_bound() -> Result<(), String> {
    let hosted = super::guided(5)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let campaign_core::NextDecision::Stop(stop) =
        crate::suite::identity::observed(super::adapter_plan(&held)?)?
    else {
        return Err(String::from("expected the host-capacity stop"));
    };
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: campaign_choregraph::control_capacity_reason_id(),
        }
    );
    let event =
        crate::suite::identity::observed(campaign_core::admit_stop(held.state(), stop.reason))?;
    let publication = crate::suite::identity::observed(hosted.publish(&held, event.clone()))?;
    let crashed = super::crashed(&hosted, &publication);
    assert_eq!(crashed.graph.len(), 5);
    let recovered = crate::suite::identity::observed(crashed.validate())?;
    assert!(recovered.state().stopped.is_none());
    let refused = Err(super::pending(&publication));
    assert_eq!(super::adapter_plan(&recovered)?.map(|_| ()), refused);
    let other = crate::suite::integrity::consumer_stop(recovered.state(), b"pending-other-stop")?;
    assert_eq!(crashed.publish(&recovered, other).map(|_| ()), refused);
    assert_decoded(&crashed, &recovered, &publication, &event)?;
    let republished = crate::suite::identity::observed(crashed.publish(&recovered, event))?;
    assert_eq!(
        republished.history_event_id(),
        publication.history_event_id()
    );
    assert_eq!(republished.graph_after_admission(), &crashed.graph);
    let stopped_hosted = crashed.committed(&republished);
    let stopped = crate::suite::identity::observed(stopped_hosted.advance(&recovered))?;
    let recorded = stopped
        .state()
        .stopped
        .clone()
        .ok_or("the stop is recorded")?;
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &recovered,
            &crate::suite::identity::observed(stopped_hosted.request())?
        ),
        Ok(None)
    );
    assert_eq!(
        super::adapter_plan(&stopped)?,
        Ok(campaign_core::NextDecision::Stop(Box::new(recorded)))
    );
    Ok(())
}

/// A consumer stop's draft held over an executed selection at a graph bound of seven
/// events refuses the planner and the selection's result alike by name; completing the stop first
/// lets the result settle within the bound.
#[test]
// r[verify campaign.publication_fence.pending_recovery]
fn a_held_stop_is_completed_before_the_outstanding_result() -> Result<(), String> {
    let hosted = super::guided(7)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let (hosted, _publication) = hosted.append(crate::suite::selection::event(&selection))?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"pending-held-stop")?;
    let stop_publication = crate::suite::identity::observed(hosted.publish(&held, stop.clone()))?;
    let crashed = super::crashed(&hosted, &stop_publication);
    let recovered = crate::suite::identity::observed(crashed.validate())?;
    let refused = Err(super::pending(&stop_publication));
    assert_eq!(super::adapter_plan(&recovered)?.map(|_| ()), refused);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        recovered.state(),
        crate::suite::selection::result_for(&recovered, &selection),
    ))?;
    assert_eq!(crashed.publish(&recovered, result).map(|_| ()), refused);
    assert_decoded(&crashed, &recovered, &stop_publication, &stop)?;
    let republished = crate::suite::identity::observed(crashed.publish(&recovered, stop))?;
    let stopped = crashed.committed(&republished);
    let stopped_view = crate::suite::identity::observed(stopped.advance(&recovered))?;
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        stopped_view.state(),
        crate::suite::selection::result_for(&stopped_view, &selection),
    ))?;
    let (settled, _) = stopped.append(result)?;
    assert_eq!(settled.graph.len(), 7);
    let settled_view = crate::suite::identity::observed(settled.validate())?;
    assert!(settled_view.state().outstanding.is_empty());
    assert!(settled_view.state().stopped.is_some());
    Ok(())
}

/// A draft on the branch head that replays a control event the lineage already accepted can never
/// be republished there: the planner refuses it by name and `pending_control_draft` answers
/// `DuplicateControlEvent`; a request that rewinds the held projection is refused as a
/// non-fast-forward before any draft is decoded.
#[test]
// r[verify campaign.publication_fence.pending_recovery]
fn an_unrecoverable_or_stale_pending_draft_is_refused_with_its_typed_error() -> Result<(), String> {
    let hosted = super::guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let accepted = &hosted.payloads[1].control_event;
    let envelope = crate::suite::configured::Envelope::canonical();
    let replay = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        accepted,
        envelope.schema,
        envelope.adapter,
        envelope.domain,
        Some(hosted.branch.head),
    ))?;
    let admitted = crate::suite::identity::observed(hosted.graph.admit(replay))?;
    let mut replayed = hosted.clone();
    replayed.graph = admitted.graph;
    let view = crate::suite::identity::observed(replayed.validate())?;
    assert_eq!(
        super::adapter_plan(&view)?.map(|_| ()),
        Err(campaign_choregraph::AdapterError::PendingControlDraft {
            event_id: admitted.event
        })
    );
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &view,
            &crate::suite::identity::observed(replayed.request())?
        ),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::DuplicateControlEvent
        ))
    );
    let selection = crate::suite::selection::select_from_current(&held)?;
    let (advanced, _) = hosted
        .clone()
        .append(crate::suite::selection::event(&selection))?;
    let later = crate::suite::identity::observed(advanced.validate())?;
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &later,
            &crate::suite::identity::observed(hosted.request())?
        ),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    Ok(())
}
