//! The reserve of a pending draft: a projection validated while a Campaign control draft is
//! pending on the branch head counts that draft as a selection (its lineage event and one
//! settlement) in the reserve it exposes, so a host whose other appends stay within
//! `graph_room` can always complete the pending publication, settle every outstanding selection,
//! and publish the stop, while a host that ignores the exposed room is refused with the typed
//! capacity error.

/// The reserve a validated projection exposes: control events and graph events it still owes,
/// and the graph room left to other writers.
fn exposed(held: &campaign_choregraph::ValidatedControlProjection) -> (u32, u32, u32) {
    (
        held.reserved_control_events(),
        held.reserved_graph_events(),
        held.graph_room(),
    )
}

/// A guided campaign with two candidates under a graph of `graph_events` total events.
fn two_candidates(graph_events: u32) -> Result<crate::suite::configured::Hosted, String> {
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
        2,
    )
}

/// A first selection committed and granted execution, and a second selection's draft admitted
/// without its branch update (the host crashed), under a graph of `graph_events` events.
struct CrashedSecond {
    committed: crate::suite::configured::Hosted,
    first: campaign_core::SelectionDecision,
    second: campaign_core::SelectionDecision,
    publication: campaign_choregraph::ControlPublication,
}

fn crashed_second(graph_events: u32) -> Result<CrashedSecond, String> {
    let hosted = two_candidates(graph_events)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let first = crate::suite::selection::select_from_current(&held)?;
    let (committed, first_publication) = hosted.append(crate::suite::selection::event(&first))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &first,
            &first_publication,
            &crate::suite::selection::exact_durable_evidence(&first_publication),
        ),
        Ok(true),
        "the first selection executes"
    );
    let held = crate::suite::identity::observed(committed.validate())?;
    assert_eq!(
        held.state().outstanding.len(),
        1,
        "the first selection executes"
    );
    let second = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        committed.publish(&held, crate::suite::selection::event(&second)),
    )?;
    Ok(CrashedSecond {
        committed,
        first,
        second,
        publication,
    })
}

/// Republishes the pending `selection` of `publication` on `written` and commits it.
fn republished(
    written: &crate::suite::configured::Hosted,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
) -> Result<crate::suite::configured::Hosted, String> {
    let recovered = crate::suite::identity::observed(written.validate())?;
    let republication = crate::suite::identity::observed(
        written.publish(&recovered, crate::suite::selection::event(selection)),
    )?;
    assert_eq!(
        republication.history_event_id(),
        publication.history_event_id()
    );
    assert_eq!(republication.graph_after_admission(), &written.graph);
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            selection,
            &republication,
            &crate::suite::selection::exact_durable_evidence(&republication),
        ),
        Ok(true)
    );
    Ok(written.committed(&republication))
}

/// At graph limits of seven to nine events, the draft of a selection admitted on a four-event
/// history leaves `limit - 7` events of room: every projection validated while it is pending
/// reserves its lineage event, its settlement, and the stop. Another writer appending exactly
/// that room leaves the pending selection completable, and once committed its result, its
/// closure, and the stop each still publish.
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.capacity]
fn a_pending_selection_reserves_its_settlement_in_the_exposed_room() -> Result<(), String> {
    for (limit, room, idle_room) in [(7, 0, 2), (8, 1, 3), (9, 2, 4)] {
        let hosted = super::guided(limit)?;
        assert_eq!(hosted.graph.len(), 4);
        let held = crate::suite::identity::observed(hosted.validate())?;
        assert_eq!(
            exposed(&held),
            (1, 1, idle_room),
            "the stop at limit {limit}"
        );
        let selection = crate::suite::selection::select_from_current(&held)?;
        let publication = crate::suite::identity::observed(
            hosted.publish(&held, crate::suite::selection::event(&selection)),
        )?;
        let crashed = super::crashed(&hosted, &publication);
        let pending = crate::suite::identity::observed(crashed.validate())?;
        assert!(pending.state().outstanding.is_empty());
        assert_eq!(
            exposed(&pending),
            (3, 2, room),
            "the pending selection's event and settlement at limit {limit}"
        );
        let granted = crate::suite::identity::observed(hosted.committed(&publication).validate())?;
        assert_eq!(
            exposed(&granted),
            (2, 2, room),
            "the same reserve once committed"
        );
        let written = crate::suite::graph_reserve::after_foreign_writes(crashed, room)?;
        let recovered = crate::suite::identity::observed(written.validate())?;
        assert_eq!(exposed(&recovered), (3, 2, 0));
        super::reserve_is_intact(
            &republished(&written, &selection, &publication)?,
            &selection,
        )?;
    }
    Ok(())
}

/// A host that ignores the exposed room (one exploration event at a room of zero) is refused
/// with the typed capacity error: republishing the pending selection is `GraphCapacityExhausted`
/// and every other event `PendingControlDraft`; had the compare-and-set committed, the granted
/// selection's result, its closure, and the stop are all `GraphCapacityExhausted`, and the planner
/// refuses with the same error instead of waiting for results that could never be published.
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.capacity]
fn appending_beyond_the_exposed_room_is_refused_with_the_capacity_error() -> Result<(), String> {
    let hosted = super::guided(7)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let crashed = super::crashed(&hosted, &publication);
    assert_eq!(
        crate::suite::identity::observed(crashed.validate())?.graph_room(),
        0
    );
    let written = crate::suite::graph_reserve::after_foreign_writes(crashed, 1)?;
    let exhausted = Err(campaign_choregraph::AdapterError::GraphCapacityExhausted);
    let recovered = crate::suite::identity::observed(written.validate())?;
    assert_eq!(exposed(&recovered), (3, 2, 0));
    assert_eq!(
        super::adapter_plan(&recovered)?.map(|_| ()),
        Err(super::pending(&publication))
    );
    assert_eq!(
        written
            .publish(&recovered, crate::suite::selection::event(&selection))
            .map(|_| ()),
        exhausted
    );
    let stop = crate::suite::integrity::consumer_stop(recovered.state(), b"reserve-blocked-stop")?;
    assert_eq!(
        written.publish(&recovered, stop).map(|_| ()),
        Err(super::pending(&publication))
    );
    let mut committed = hosted.committed(&publication);
    committed.graph = written.graph.clone();
    let granted = crate::suite::identity::observed(committed.validate())?;
    assert_eq!(super::adapter_plan(&granted)?.map(|_| ()), exhausted);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        granted.state(),
        crate::suite::selection::result_for(&granted, &selection),
    ))?;
    let stop = crate::suite::integrity::consumer_stop(granted.state(), b"reserve-stranded-stop")?;
    for event in [result, super::closure_of(&granted, &selection)?, stop] {
        assert_eq!(committed.publish(&granted, event).map(|_| ()), exhausted);
    }
    Ok(())
}

/// With a first selection executing and a second selection's draft pending after a crash (seven
/// events), the reserve holds both settlements and the stop, so the room is `limit - 10`.
/// Another writer appending exactly that room leaves the second selection completable, and then
/// the first selection's result, the second's closure, and the stop publish, filling the graph.
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.capacity]
fn an_executed_selection_settles_after_a_crashed_selection_within_the_room() -> Result<(), String> {
    for (limit, room) in [(10, 0), (11, 1)] {
        let crashed = crashed_second(limit)?;
        let hosted = super::crashed(&crashed.committed, &crashed.publication);
        let pending = crate::suite::identity::observed(hosted.validate())?;
        assert_eq!(
            pending.state().outstanding.len(),
            1,
            "only the first selection"
        );
        assert_eq!(pending.graph_events(), 7);
        assert_eq!(exposed(&pending), (4, 3, room), "at limit {limit}");
        let written = crate::suite::graph_reserve::after_foreign_writes(hosted, room)?;
        let committed = republished(&written, &crashed.second, &crashed.publication)?;
        let granted = crate::suite::identity::observed(committed.validate())?;
        let result = crate::suite::identity::observed(campaign_core::admit_result(
            granted.state(),
            crate::suite::selection::result_for(&granted, &crashed.first),
        ))?;
        let (settled, _) = committed.append(result)?;
        let held = crate::suite::identity::observed(settled.validate())?;
        let (closed, _) = settled.append(super::closure_of(&held, &crashed.second)?)?;
        let held = crate::suite::identity::observed(closed.validate())?;
        let stop = crate::suite::integrity::consumer_stop(held.state(), b"reserve-final-stop")?;
        let (stopped, _) = closed.append(stop)?;
        let held = crate::suite::identity::observed(stopped.validate())?;
        assert!(held.state().stopped.is_some());
        assert_eq!(stopped.graph.len(), limit);
        assert_eq!(exposed(&held), (0, 0, 0));
    }
    Ok(())
}

/// The same crash at a graph limit of ten exposes no room; one exploration event appended anyway
/// makes the second selection's republication `GraphCapacityExhausted`, and the first selection's
/// result, its closure, and every stop stay refused with `PendingControlDraft` naming the second
/// selection's draft (there is no abandonment).
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.capacity]
fn an_exploration_event_beyond_the_room_blocks_the_crashed_selection() -> Result<(), String> {
    let crashed = crashed_second(10)?;
    let hosted = super::crashed(&crashed.committed, &crashed.publication);
    assert_eq!(
        crate::suite::identity::observed(hosted.validate())?.graph_room(),
        0
    );
    let written = crate::suite::graph_reserve::after_foreign_writes(hosted, 1)?;
    let recovered = crate::suite::identity::observed(written.validate())?;
    let pending = Err(super::pending(&crashed.publication));
    assert_eq!(super::adapter_plan(&recovered)?.map(|_| ()), pending);
    assert_eq!(
        written
            .publish(&recovered, crate::suite::selection::event(&crashed.second))
            .map(|_| ()),
        Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
    );
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        recovered.state(),
        crate::suite::selection::result_for(&recovered, &crashed.first),
    ))?;
    let stop = crate::suite::integrity::consumer_stop(recovered.state(), b"reserve-wedged-stop")?;
    for event in [result, super::closure_of(&recovered, &crashed.first)?, stop] {
        assert_eq!(written.publish(&recovered, event).map(|_| ()), pending);
    }
    Ok(())
}
