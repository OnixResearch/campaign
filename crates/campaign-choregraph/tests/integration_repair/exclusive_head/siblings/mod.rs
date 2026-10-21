//! Racing drafts: two publishers from one held projection leave two sibling drafts on the head
//! (hosts serialize publication per control branch, so this is a host fault). The reserve counts
//! both siblings honestly, the adapter only republishes a sibling whose settlement and stop still
//! fit, and the planner agrees with publication once a raced compare-and-set strands the winner.

/// A guided campaign under a graph of `graph_events` events whose graph holds a selection draft and a racing
/// stop draft, both children of the held head.
struct Raced {
    hosted: crate::suite::configured::Hosted,
    racing: crate::suite::configured::Hosted,
    selection: campaign_core::SelectionDecision,
    publication: campaign_choregraph::ControlPublication,
    stop: campaign_core::CampaignControlEvent,
}

fn raced(graph_events: u32) -> Result<Raced, String> {
    let hosted = super::guided(graph_events)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"racing-stop")?;
    let stop_publication = crate::suite::identity::observed(hosted.publish(&held, stop.clone()))?;
    let merged = crate::suite::identity::observed(
        publication
            .graph_after_admission()
            .admit(stop_publication.draft().clone()),
    )?;
    assert_eq!(
        stop_publication.branch_plan().before,
        publication.branch_plan().before
    );
    assert_ne!(
        stop_publication.history_event_id(),
        publication.history_event_id()
    );
    let racing = crate::suite::configured::Hosted {
        graph: merged.graph,
        ..hosted.clone()
    };
    Ok(Raced {
        hosted,
        racing,
        selection,
        publication,
        stop,
    })
}

/// Both sibling drafts are reserved (control 5, graph 3), so the exposed room is
/// `limit - 6 - 3`, floored at zero. A host that appends exactly that room completes either
/// sibling: the selection republishes, settles, and stops, or the stop republishes; at limits 8
/// to 10 no capacity error occurs.
#[test]
// r[verify campaign.publication_fence.pending_several]
// r[verify campaign.publication_fence.pending_reserve]
fn racing_drafts_are_reserved_and_either_completes_within_the_room() -> Result<(), String> {
    for (limit, room) in [(8_u32, 0_u32), (9, 0), (10, 1)] {
        for commit_selection in [true, false] {
            let race = raced(limit)?;
            let view = crate::suite::identity::observed(race.racing.validate())?;
            let exposed = (
                view.reserved_control_events(),
                view.reserved_graph_events(),
                view.graph_room(),
            );
            assert_eq!(exposed, (5, 3, room), "limit {limit}");
            let written = crate::suite::graph_reserve::after_foreign_writes(race.racing, room)?;
            let recovered = crate::suite::identity::observed(written.validate())?;
            if commit_selection {
                let republished = crate::suite::identity::observed(
                    written.publish(&recovered, crate::suite::selection::event(&race.selection)),
                )?;
                super::reserve_is_intact(&written.committed(&republished), &race.selection)?;
            } else {
                let republished =
                    crate::suite::identity::observed(written.publish(&recovered, race.stop))?;
                let stopped =
                    crate::suite::identity::observed(written.committed(&republished).validate())?;
                assert!(stopped.state().stopped.is_some());
            }
        }
    }
    Ok(())
}

/// The round-9 reproducer at graph limit 7: the two siblings overcommit the graph (6 events and a
/// reserve of 3). The adapter republishes the stop but refuses to republish the selection
/// (`GraphCapacityExhausted`), so it never grants a selection whose settlement cannot publish. A
/// host whose raced compare-and-set committed the selection anyway (outside the adapter, against
/// the serialization rule) finds its result, closure, and stop refused with
/// `GraphCapacityExhausted`, and the planner refuses with the same error instead of waiting for
/// results.
#[test]
// r[verify campaign.publication_fence.pending_several]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn racing_drafts_beyond_the_bound_grant_nothing_that_cannot_settle() -> Result<(), String> {
    let race = raced(7)?;
    let exhausted = Err(campaign_choregraph::AdapterError::GraphCapacityExhausted);
    let view = crate::suite::identity::observed(race.racing.validate())?;
    assert_eq!(view.graph_room(), 0);
    assert_eq!(
        race.racing
            .publish(&view, crate::suite::selection::event(&race.selection))
            .map(|_| ()),
        exhausted
    );
    let stop = crate::suite::identity::observed(race.racing.publish(&view, race.stop.clone()))?;
    let stopped = crate::suite::identity::observed(race.racing.committed(&stop).validate())?;
    assert!(stopped.state().stopped.is_some());
    let mut committed = race.hosted.committed(&race.publication);
    committed.graph = race.racing.graph.clone();
    let granted = crate::suite::identity::observed(committed.validate())?;
    assert_eq!(super::adapter_plan(&granted)?.map(|_| ()), exhausted);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        granted.state(),
        crate::suite::selection::result_for(&granted, &race.selection),
    ))?;
    let own_stop = crate::suite::integrity::consumer_stop(granted.state(), b"racing-own-stop")?;
    for event in [
        result,
        super::closure_of(&granted, &race.selection)?,
        own_stop,
    ] {
        assert_eq!(committed.publish(&granted, event).map(|_| ()), exhausted);
    }
    Ok(())
}
