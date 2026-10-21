//! Several drafts on one head and the documented no-abandonment residual: every refusal and
//! `pending_control_draft` name the first pending draft in Choregraph event-identity order,
//! republishing any identical draft stays allowed, and a draft the host cannot complete blocks the
//! head with `PendingControlDraft` naming it.

/// Two racing publishers from one held projection each admitted a draft on its head (a selection
/// and a consumer stop): the planner, a third event, and `pending_control_draft` all name the
/// smaller event identity. Republishing either identical draft is allowed; once the selection's
/// republication commits, the stop's draft is no longer a child of the head, nothing is pending,
/// and the selection settles.
#[test]
// r[verify campaign.publication_fence.pending_several]
// r[verify campaign.publication_fence.republication]
fn several_pending_drafts_are_named_in_identity_order_and_each_republishes() -> Result<(), String> {
    let hosted = super::guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let selected = crate::suite::selection::event(&selection);
    let publication = crate::suite::identity::observed(hosted.publish(&held, selected.clone()))?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"residual-racing-stop")?;
    let stop_publication = crate::suite::identity::observed(hosted.publish(&held, stop.clone()))?;
    let raced = crate::suite::identity::observed(
        publication
            .graph_after_admission()
            .admit(stop_publication.draft().clone()),
    )?;
    let mut racing = hosted.clone();
    racing.graph = raced.graph;
    let first = publication
        .history_event_id()
        .min(stop_publication.history_event_id());
    let named = Err(campaign_choregraph::AdapterError::PendingControlDraft { event_id: first });
    let recovered = crate::suite::identity::observed(racing.validate())?;
    assert_eq!(super::adapter_plan(&recovered)?.map(|_| ()), named);
    let third = crate::suite::integrity::consumer_stop(held.state(), b"residual-third-stop")?;
    assert_eq!(racing.publish(&recovered, third).map(|_| ()), named);
    let draft = crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
        &held,
        &crate::suite::identity::observed(racing.request())?,
    ))?
    .ok_or("a draft is pending")?;
    assert_eq!(draft.history_event_id(), first);
    let expected = if first == publication.history_event_id() {
        selected.event_id()
    } else {
        stop.event_id()
    };
    assert_eq!(draft.control_event_id(), expected);
    let republished_stop = crate::suite::identity::observed(racing.publish(&recovered, stop))?;
    assert_eq!(
        republished_stop.history_event_id(),
        stop_publication.history_event_id()
    );
    let republished = crate::suite::identity::observed(racing.publish(&recovered, selected))?;
    assert_eq!(
        republished.history_event_id(),
        publication.history_event_id()
    );
    let committed = racing.committed(&republished);
    assert_eq!(
        campaign_choregraph::pending_control_draft(
            &recovered,
            &crate::suite::identity::observed(committed.request())?
        ),
        Ok(None)
    );
    super::reserve_is_intact(&committed, &selection)
}

/// Documented residual (no abandonment): one draft under the Campaign envelope whose parent is
/// the branch head but whose payload identity is not its named control event's blocks the branch
/// with no bound near. The planner, the executed selection's result and closure, the host's own
/// stop, and republication of the named event are all refused with `PendingControlDraft` naming
/// the forged draft; `pending_control_draft` decodes it and names the event, whose republication
/// builds another draft.
#[test]
// r[verify campaign.publication_fence.pending_blocked]
fn a_draft_whose_payload_is_not_its_named_event_blocks_the_branch() -> Result<(), String> {
    let hosted = super::guided(crate::suite::constants::MAX_EVENTS)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let (hosted, publication) = hosted.append(crate::suite::selection::event(&selection))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &crate::suite::selection::exact_durable_evidence(&publication),
        ),
        Ok(true)
    );
    let held = crate::suite::identity::observed(hosted.validate())?;
    let named = crate::suite::integrity::consumer_stop(held.state(), b"residual-named-stop")?;
    let envelope = crate::suite::configured::Envelope::canonical();
    let mut forged = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &named,
        envelope.schema,
        envelope.adapter,
        envelope.domain,
        Some(hosted.branch.head),
    ))?;
    forged.payload_identity = campaign_choregraph::Digest::from_bytes([7; 32]);
    let admitted = crate::suite::identity::observed(hosted.graph.admit(forged))?;
    let mut blocked = hosted.clone();
    blocked.graph = admitted.graph;
    let view = crate::suite::identity::observed(blocked.validate())?;
    let refused = Err(campaign_choregraph::AdapterError::PendingControlDraft {
        event_id: admitted.event,
    });
    assert_eq!(super::adapter_plan(&view)?.map(|_| ()), refused);
    let decoded = crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
        &held,
        &crate::suite::identity::observed(blocked.request())?,
    ))?
    .ok_or("the forged draft is pending")?;
    assert_eq!(decoded.history_event_id(), admitted.event);
    assert_eq!(decoded.control_event_id(), named.event_id());
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        view.state(),
        crate::suite::selection::result_for(&view, &selection),
    ))?;
    let own_stop = crate::suite::integrity::consumer_stop(view.state(), b"residual-own-stop")?;
    for event in [
        result,
        super::closure_of(&view, &selection)?,
        own_stop,
        named,
    ] {
        assert_eq!(blocked.publish(&view, event).map(|_| ()), refused);
    }
    Ok(())
}
