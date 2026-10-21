//! Every uncommitted Campaign control draft descending from the branch head is reserved: a chain
//! a pipelining host left below the head (outside the host obligation to publish only on durably
//! committed branch values) is counted draft by draft (its event and one settlement each) as a
//! defence and completed in chain order where it fits, and republishing a draft checks the drafts
//! chained below it. A chain of selections is counted exactly; `residual` pins the chains that
//! count is conservative for.

mod residual;

/// A guided campaign with two candidates under a graph of `graph_events` total events.
fn guided_pair(graph_events: u32) -> Result<crate::suite::configured::Hosted, String> {
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

/// A pipelining host's crash: selection S1 published (draft D1) and, on D1's planned branch
/// value, S2 (draft D2, parent D1), neither branch update durable. The durable history keeps the
/// branch at the head the drafts descend from.
struct Crashed {
    durable: crate::suite::configured::Hosted,
    first: campaign_core::SelectionDecision,
    second: campaign_core::SelectionDecision,
    d1: campaign_choregraph::ControlPublication,
    d2: campaign_choregraph::ControlPublication,
}

fn crashed_pair(graph_events: u32) -> Result<Crashed, String> {
    let hosted = guided_pair(graph_events)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let first = crate::suite::selection::select_from_current(&held)?;
    let d1 = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&first)),
    )?;
    let pipelined = hosted.committed(&d1);
    let held1 = crate::suite::identity::observed(pipelined.validate())?;
    let second = crate::suite::selection::select_from_current(&held1)?;
    let d2 = crate::suite::identity::observed(
        pipelined.publish(&held1, crate::suite::selection::event(&second)),
    )?;
    assert_eq!(d2.branch_plan().before.head, d1.history_event_id());
    let durable = crate::suite::configured::Hosted {
        graph: d2.graph_after_admission().clone(),
        ..hosted
    };
    assert!(durable.graph.contains(d1.history_event_id()));
    Ok(Crashed {
        durable,
        first,
        second,
        d1,
        d2,
    })
}

fn named(
    held: &campaign_choregraph::ValidatedControlProjection,
    hosted: &crate::suite::configured::Hosted,
) -> Result<campaign_choregraph::EventId, String> {
    let request = crate::suite::identity::observed(hosted.request())?;
    Ok(
        crate::suite::identity::observed(campaign_choregraph::pending_control_draft(
            held, &request,
        ))?
        .ok_or("a pending draft")?
        .history_event_id(),
    )
}

/// The round-9 reproducer: at graph limit 10 the durable view used to reserve D1 only and expose
/// a room of 1; a writer filling it left D2 unpublishable after S1 executed. The view now
/// reserves both drafts (control 5, graph 3) and exposes no room; recovery names D1 first, D2
/// presented before D1 commits is `PublicationFenceMismatch`, and the planner names D1. Within
/// the room D1 republishes, S1 executes, D2 becomes the named draft and republishes, and S1's
/// result, S2's closure, and the stop fill the graph. One event beyond the room makes D1's
/// republication `GraphCapacityExhausted` (D2 chained below it no longer fits), so S1 is never
/// granted.
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.pending_recovery]
// r[verify campaign.publication_fence.pending_several]
fn every_pending_draft_is_reserved_and_completed_in_order() -> Result<(), String> {
    for beyond in [0_u32, 1] {
        let pipeline = crashed_pair(10)?;
        let view = crate::suite::identity::observed(pipeline.durable.validate())?;
        assert_eq!(view.graph_events(), 7);
        let exposed = (
            view.reserved_control_events(),
            view.reserved_graph_events(),
            view.graph_room(),
        );
        assert_eq!(exposed, (5, 3, 0));
        assert_eq!(
            named(&view, &pipeline.durable)?,
            pipeline.d1.history_event_id()
        );
        let pending_d1 = Err(campaign_choregraph::AdapterError::PendingControlDraft {
            event_id: pipeline.d1.history_event_id(),
        });
        assert_eq!(super::adapter_plan(&view)?.map(|_| ()), pending_d1);
        assert_eq!(
            pipeline
                .durable
                .publish(&view, crate::suite::selection::event(&pipeline.second))
                .map(|_| ()),
            Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
        );
        let written =
            crate::suite::graph_reserve::after_foreign_writes(pipeline.durable.clone(), beyond)?;
        let recovered = crate::suite::identity::observed(written.validate())?;
        let r1 = written.publish(&recovered, crate::suite::selection::event(&pipeline.first));
        if beyond > 0 {
            assert_eq!(
                r1.map(|_| ()),
                Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
            );
            continue;
        }
        let r1 = crate::suite::identity::observed(r1)?;
        assert_eq!(r1.history_event_id(), pipeline.d1.history_event_id());
        let committed = written.committed(&r1);
        assert_eq!(
            campaign_choregraph::can_execute_selection_after_publication(
                &pipeline.first,
                &r1,
                &crate::suite::selection::exact_durable_evidence(&r1),
            ),
            Ok(true)
        );
        let granted = crate::suite::identity::observed(committed.validate())?;
        assert_eq!(named(&granted, &committed)?, pipeline.d2.history_event_id());
        assert_eq!(
            super::adapter_plan(&granted)?.map(|_| ()),
            Err(campaign_choregraph::AdapterError::PendingControlDraft {
                event_id: pipeline.d2.history_event_id(),
            })
        );
        completes_after_the_first(&committed, &granted, &pipeline)?;
    }
    Ok(())
}

/// With D1 committed (`committed`, validated as `granted`), D2 republishes, and S1's result, S2's
/// closure, and the stop fill the graph of 10 events.
fn completes_after_the_first(
    committed: &crate::suite::configured::Hosted,
    granted: &campaign_choregraph::ValidatedControlProjection,
    pipeline: &Crashed,
) -> Result<(), String> {
    let r2 = crate::suite::identity::observed(
        committed.publish(granted, crate::suite::selection::event(&pipeline.second)),
    )?;
    assert_eq!(r2.history_event_id(), pipeline.d2.history_event_id());
    let committed = committed.committed(&r2);
    let held = crate::suite::identity::observed(committed.validate())?;
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        held.state(),
        crate::suite::selection::result_for(&held, &pipeline.first),
    ))?;
    let (settled, _) = committed.append(result)?;
    let held = crate::suite::identity::observed(settled.validate())?;
    let (closed, _) = settled.append(super::closure_of(&held, &pipeline.second)?)?;
    let held = crate::suite::identity::observed(closed.validate())?;
    let (stopped, _) = closed.append(crate::suite::integrity::consumer_stop(
        held.state(),
        b"pipelined-stop",
    )?)?;
    assert_eq!(stopped.graph.len(), 10);
    Ok(())
}

/// Republication counts the drafts chained below the republished draft against the projection
/// bound as well: with the host bound at exactly the durable view's lineage plus its recorded
/// control reserve D1 republishes, and one event lower it is `ProjectionCapacityExhausted`,
/// although D1 alone (its event, its settlement, and the stop) would fit.
#[test]
// r[verify campaign.publication_fence.pending_reserve]
// r[verify campaign.publication_fence.capacity]
fn republication_counts_the_drafts_below_against_the_projection_bound() -> Result<(), String> {
    let pipeline = crashed_pair(crate::suite::constants::MAX_EVENTS)?;
    let view = crate::suite::identity::observed(pipeline.durable.validate())?;
    let exact = u32::try_from(view.control_events().len())
        .map_err(|error| error.to_string())?
        .checked_add(view.reserved_control_events())
        .ok_or("bound overflow")?;
    assert_eq!(view.reserved_control_events(), 5);
    let below = exact.checked_sub(1).ok_or("bound underflow")?;
    for (maximum_events, outcome) in [
        (exact, Ok(())),
        (
            below,
            Err(campaign_choregraph::AdapterError::ProjectionCapacityExhausted),
        ),
    ] {
        let bounded = crate::suite::configured::Hosted {
            maximum_events,
            ..pipeline.durable.clone()
        };
        let held = crate::suite::identity::observed(bounded.validate())?;
        assert_eq!(
            bounded
                .publish(&held, crate::suite::selection::event(&pipeline.first))
                .map(|_| ()),
            outcome,
            "bound {maximum_events}"
        );
    }
    Ok(())
}
