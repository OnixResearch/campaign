//! The pipelined-chain residual (D91). Hosts publish a control event only on a durably committed
//! branch value; the reserve still counts every draft below the head, each as a selection, as a
//! defence against hosts that pipeline. That count is conservative: a chained draft that settles a
//! selection or records the stop fills a slot the reserve already holds, yet republishing the
//! draft above it counts the chained draft once more as a selection. A pipelining host's chain
//! can therefore be refused at the bound and wedge the head (every other event is refused with
//! `PendingControlDraft` naming the undeliverable draft). These tests pin that typed behaviour
//! and show that a non-pipelining host completes the same events at the same bound. A chained
//! draft that is not a selection is itself refused from the durable head with the replay error
//! `StaleState`, because its event names the state after the draft above it.

mod obligation;

/// The host bounds of a pipelined history: the graph's own event limit and the host projection
/// bound.
#[derive(Clone, Copy)]
struct Bounds {
    graph_events: u32,
    maximum_events: u32,
}

impl Bounds {
    /// A graph limit of `graph_events` under the suite's projection bound.
    const fn graph(graph_events: u32) -> Self {
        Self {
            graph_events,
            maximum_events: crate::suite::constants::MAX_EVENTS,
        }
    }
}

/// A guided campaign with one candidate whose selection S is published, committed, and granted,
/// under `bounds`.
fn granted(
    bounds: Bounds,
) -> Result<
    (
        crate::suite::configured::Hosted,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let hosted = crate::suite::configured::Hosted {
        maximum_events: bounds.maximum_events,
        ..super::super::guided(bounds.graph_events)?
    };
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
    Ok((hosted, selection))
}

/// A pipelining host's crash after it published the consumer stop over the granted selection S
/// (draft `stop`) and, on the stop's planned branch value, S's post-stop result (draft `result`,
/// parent `stop`); neither branch update is durable.
struct StopThenResult {
    durable: crate::suite::configured::Hosted,
    selection: campaign_core::SelectionDecision,
    stop: campaign_choregraph::ControlPublication,
    result: campaign_choregraph::ControlPublication,
}

fn stop_then_result(bounds: Bounds) -> Result<StopThenResult, String> {
    let (hosted, selection) = granted(bounds)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let stop_event = crate::suite::integrity::consumer_stop(held.state(), b"pipelined-stop")?;
    let stop = crate::suite::identity::observed(hosted.publish(&held, stop_event))?;
    let pipelined = hosted.committed(&stop);
    let stopped = crate::suite::identity::observed(pipelined.advance(&held))?;
    assert!(stopped.state().stopped.is_some());
    let result_event = crate::suite::identity::observed(campaign_core::admit_result(
        stopped.state(),
        crate::suite::selection::result_for(&stopped, &selection),
    ))?;
    let result = crate::suite::identity::observed(pipelined.publish(&stopped, result_event))?;
    assert_eq!(result.branch_plan().before.head, stop.history_event_id());
    Ok(StopThenResult {
        durable: crate::suite::configured::Hosted {
            graph: result.graph_after_admission().clone(),
            ..hosted
        },
        selection,
        stop,
        result,
    })
}

fn exposed(held: &campaign_choregraph::ValidatedControlProjection) -> (u32, u32, u32) {
    (
        held.reserved_control_events(),
        held.reserved_graph_events(),
        held.graph_room(),
    )
}

/// Requires planning and every other event on the durable view (S's result and closure, and a
/// stop, less the undeliverable draft's own event) to be refused naming `draft`.
fn wedged(
    durable: &crate::suite::configured::Hosted,
    view: &campaign_choregraph::ValidatedControlProjection,
    selection: &campaign_core::SelectionDecision,
    draft: &campaign_choregraph::ControlPublication,
) -> Result<(), String> {
    let blocked = Err(super::super::pending(draft));
    assert_eq!(super::super::adapter_plan(view)?.map(|_| ()), blocked);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        view.state(),
        crate::suite::selection::result_for(view, selection),
    ))?;
    let stop = crate::suite::integrity::consumer_stop(view.state(), b"durable-view-stop")?;
    for event in [result, super::super::closure_of(view, selection)?, stop] {
        if &event != draft.control_event() {
            assert_eq!(durable.publish(view, event).map(|_| ()), blocked);
        }
    }
    Ok(())
}

/// Residual, graph bound: over a granted selection S at graph limits 7 and 8, a pipelining host's
/// chain "stop, then S's result" leaves the stop draft pending with the result chained below it.
/// Republishing the stop counts the chained result as a selection on top of S's settlement and
/// is `GraphCapacityExhausted`, so S never settles and the stop never records. At limit 9 the
/// chain completes in order.
#[test]
// r[verify campaign.publication_fence.durable_base]
// r[verify campaign.publication_fence.pending_reserve]
fn stop_then_result_wedges_at_the_graph_bound() -> Result<(), String> {
    for (limit, is_wedged) in [(7_u32, true), (8, true), (9, false)] {
        let chain = stop_then_result(Bounds::graph(limit))?;
        let view = crate::suite::identity::observed(chain.durable.validate())?;
        assert_eq!(view.graph_events(), 7);
        assert_eq!(exposed(&view), (6, 4, limit.saturating_sub(11)));
        assert_eq!(
            super::named(&view, &chain.durable)?,
            chain.stop.history_event_id()
        );
        let republished = chain
            .durable
            .publish(&view, chain.stop.control_event().clone());
        if is_wedged {
            assert_eq!(
                republished.map(|_| ()),
                Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
            );
            wedged(&chain.durable, &view, &chain.selection, &chain.stop)?;
            continue;
        }
        let committed = chain
            .durable
            .committed(&crate::suite::identity::observed(republished)?);
        let stopped = crate::suite::identity::observed(committed.validate())?;
        let settled = committed.committed(&crate::suite::identity::observed(
            committed.publish(&stopped, chain.result.control_event().clone()),
        )?);
        let done = crate::suite::identity::observed(settled.validate())?;
        assert!(done.state().stopped.is_some());
        assert!(done.state().outstanding.is_empty());
    }
    Ok(())
}

/// Residual, projection bound: the same chain under a host projection bound of 6 or 7 (graph
/// limit 16) makes the stop's republication `ProjectionCapacityExhausted`; at bound 8 it
/// completes.
#[test]
// r[verify campaign.publication_fence.durable_base]
// r[verify campaign.publication_fence.pending_reserve]
fn stop_then_result_wedges_at_the_projection_bound() -> Result<(), String> {
    for (bound, is_wedged) in [(6_u32, true), (7, true), (8, false)] {
        let chain = stop_then_result(Bounds {
            graph_events: 16,
            maximum_events: bound,
        })?;
        let view = crate::suite::identity::observed(chain.durable.validate())?;
        let republished = chain
            .durable
            .publish(&view, chain.stop.control_event().clone());
        if is_wedged {
            assert_eq!(
                republished.map(|_| ()),
                Err(campaign_choregraph::AdapterError::ProjectionCapacityExhausted)
            );
            wedged(&chain.durable, &view, &chain.selection, &chain.stop)?;
        } else {
            assert_eq!(
                crate::suite::identity::observed(republished)?.history_event_id(),
                chain.stop.history_event_id()
            );
        }
    }
    Ok(())
}

/// Residual, the other order: S's result R published and, on R's planned branch value, the stop
/// (chained below R). At graph limits 7 and 8 republishing R counts the chained stop as a
/// selection plus the stop slot and is `GraphCapacityExhausted`; at 9 the chain completes.
#[test]
// r[verify campaign.publication_fence.durable_base]
// r[verify campaign.publication_fence.pending_reserve]
fn result_then_stop_wedges_at_the_graph_bound() -> Result<(), String> {
    for (limit, is_wedged) in [(7_u32, true), (8, true), (9, false)] {
        let (hosted, selection) = granted(Bounds::graph(limit))?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let result_event = crate::suite::identity::observed(campaign_core::admit_result(
            held.state(),
            crate::suite::selection::result_for(&held, &selection),
        ))?;
        let result = crate::suite::identity::observed(hosted.publish(&held, result_event))?;
        let pipelined = hosted.committed(&result);
        let settled = crate::suite::identity::observed(pipelined.advance(&held))?;
        let stop_event =
            crate::suite::integrity::consumer_stop(settled.state(), b"pipelined-stop")?;
        let stop = crate::suite::identity::observed(pipelined.publish(&settled, stop_event))?;
        let durable = crate::suite::configured::Hosted {
            graph: stop.graph_after_admission().clone(),
            ..hosted
        };
        let view = crate::suite::identity::observed(durable.validate())?;
        assert_eq!(super::named(&view, &durable)?, result.history_event_id());
        let republished = durable.publish(&view, result.control_event().clone());
        if is_wedged {
            assert_eq!(
                republished.map(|_| ()),
                Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
            );
            wedged(&durable, &view, &selection, &result)?;
            continue;
        }
        let committed = durable.committed(&crate::suite::identity::observed(republished)?);
        let recovered = crate::suite::identity::observed(committed.validate())?;
        let stopped = committed.committed(&crate::suite::identity::observed(
            committed.publish(&recovered, stop.control_event().clone()),
        )?);
        assert!(
            crate::suite::identity::observed(stopped.validate())?
                .state()
                .stopped
                .is_some()
        );
    }
    Ok(())
}
