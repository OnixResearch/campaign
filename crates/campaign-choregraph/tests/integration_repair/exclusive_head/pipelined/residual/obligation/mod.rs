//! The host obligation the pipelined-chain residual rests on, and how a chained draft's event is
//! refused from the durable head.

/// The host obligation the residual rests on: a host that publishes only on durably committed
/// branch values completes the same events (the stop, then S's result) at graph limit 7, also
/// when it crashes between the stop's draft and its compare-and-set and recovers by
/// republishing the pending stop.
#[test]
// r[verify campaign.publication_fence.durable_base]
// r[verify campaign.publication_fence.pending_recovery]
// r[verify campaign.publication_fence.pending_reserve]
fn a_non_pipelining_host_completes_the_same_chain_at_the_bound() -> Result<(), String> {
    let (hosted, selection) = super::granted(super::Bounds::graph(7))?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let stop_event = crate::suite::integrity::consumer_stop(held.state(), b"serial-stop")?;
    let stop = crate::suite::identity::observed(hosted.publish(&held, stop_event))?;
    let crashed = super::super::super::crashed(&hosted, &stop);
    let view = crate::suite::identity::observed(crashed.validate())?;
    assert_eq!(super::exposed(&view), (4, 3, 0));
    assert_eq!(
        super::super::named(&view, &crashed)?,
        stop.history_event_id()
    );
    let recovered = crashed.committed(&crate::suite::identity::observed(
        crashed.publish(&view, stop.control_event().clone()),
    )?);
    let stopped = crate::suite::identity::observed(recovered.validate())?;
    let result_event = crate::suite::identity::observed(campaign_core::admit_result(
        stopped.state(),
        crate::suite::selection::result_for(&stopped, &selection),
    ))?;
    let done = recovered.committed(&crate::suite::identity::observed(
        recovered.publish(&stopped, result_event),
    )?);
    let done = crate::suite::identity::observed(done.validate())?;
    assert_eq!(done.graph_events(), 7);
    assert!(done.state().stopped.is_some());
    assert!(done.state().outstanding.is_empty());
    Ok(())
}

/// A chained draft that is not a selection, presented from the durable head before the draft
/// above it commits, is refused with the replay error `StaleState` (its event names the state
/// after the draft above it), not `PendingControlDraft`. A new event computed on the durable
/// head is `PendingControlDraft` naming the draft above, also when a hand-built draft of that
/// event is chained below the draft above (publication builds its draft on the head instead).
#[test]
// r[verify campaign.publication_fence.durable_base]
// r[verify campaign.publication_fence.pending_several]
fn a_chained_non_selection_draft_fails_replay_from_the_durable_branch() -> Result<(), String> {
    let chain = super::stop_then_result(super::Bounds {
        graph_events: crate::suite::constants::MAX_EVENTS,
        maximum_events: 16,
    })?;
    let view = crate::suite::identity::observed(chain.durable.validate())?;
    assert_eq!(
        chain
            .durable
            .publish(&view, chain.result.control_event().clone())
            .map(|_| ()),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::StaleState
        ))
    );
    let fresh = crate::suite::identity::observed(campaign_core::admit_result(
        view.state(),
        crate::suite::selection::result_for(&view, &chain.selection),
    ))?;
    let blocked = Err(super::super::super::pending(&chain.stop));
    assert_eq!(
        chain.durable.publish(&view, fresh.clone()).map(|_| ()),
        blocked
    );
    let binding = view.binding();
    let hand_built = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &fresh,
        binding.projection_schema_id,
        binding.adapter_id,
        binding.control_domain_id,
        Some(chain.stop.history_event_id()),
    ))?;
    let graph = crate::suite::identity::observed(chain.durable.graph.admit(hand_built))?.graph;
    let chained = crate::suite::configured::Hosted {
        graph,
        ..chain.durable.clone()
    };
    let held = crate::suite::identity::observed(chained.advance(&view))?;
    assert_eq!(
        Some(held.reserved_control_events()),
        view.reserved_control_events().checked_add(2)
    );
    assert_eq!(chained.publish(&held, fresh).map(|_| ()), blocked);
    Ok(())
}
