//! A fast-forward never goes back in graph time: a held projection advances only over a graph
//! holding every event it recorded.

fn stale<T>() -> campaign_choregraph::AdapterResult<T> {
    Err(campaign_choregraph::AdapterError::StaleGraph)
}

/// The round-9 reproducer at graph limit 7: a host holding a projection that recorded selection
/// draft D1 pending used to advance over the lagging graph without D1 and publish a stop there;
/// merged with D1 that stop stranded S1's settlement. Every path that advances the held
/// projection (advance, publication, pending-draft recovery, cache validation) now refuses the
/// older graph with `StaleGraph`, as it does a graph of as many events that lacks D1. The graph
/// the projection was validated over and a later one advance, and a cold load of the older graph
/// still validates.
#[test]
// r[verify campaign.history_projection.older_graph]
// r[verify campaign.publication_fence.pending]
fn a_graph_older_than_the_held_projection_is_refused() -> Result<(), String> {
    let hosted = crate::suite::configured::campaign::guided(
        crate::suite::configured::campaign::admitted_under(
            crate::suite::configured::campaign::root::Limits {
                graph_events: 7,
                ..crate::suite::configured::campaign::root::Limits::standard(
                    crate::suite::constants::ROOT_GENERATION,
                    crate::suite::constants::MAX_EVENTS,
                )
            },
        )?,
        1,
    )?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let publication = crate::suite::identity::observed(
        hosted.publish(&held, crate::suite::selection::event(&selection)),
    )?;
    let current = crate::suite::configured::Hosted {
        graph: publication.graph_after_admission().clone(),
        ..hosted.clone()
    };
    let view = crate::suite::identity::observed(current.validate())?;
    assert_eq!(view.graph_events(), 5);
    let same_length = crate::suite::graph_reserve::after_foreign_writes(hosted.clone(), 1)?;
    assert_eq!(same_length.graph.len(), view.graph_events());
    let stop = crate::suite::integrity::consumer_stop(view.state(), b"older-graph-stop")?;
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        view.state(),
        &crate::suite::identity::observed(hosted.binding())?,
    ))?;
    for older in [&hosted, &same_length] {
        assert_eq!(older.advance(&view), stale());
        assert_eq!(older.publish(&view, stop.clone()).map(|_| ()), stale());
        let request = crate::suite::identity::observed(older.request())?;
        assert_eq!(
            campaign_choregraph::pending_control_draft(&view, &request),
            stale()
        );
    }
    assert_eq!(hosted.validate_cache(Some(&view), &cache), stale());
    assert_eq!(
        hosted.validate_cache(None, &cache),
        Ok(view.state().clone())
    );
    assert_eq!(current.advance(&view), Ok(view.clone()));
    let later = crate::suite::graph_reserve::after_foreign_writes(current.clone(), 1)?;
    let advanced = crate::suite::identity::observed(later.advance(&view))?;
    assert_eq!(advanced.graph_events(), 6);
    assert_eq!(
        current.publish(&view, stop).map(|_| ()),
        Err(campaign_choregraph::AdapterError::PendingControlDraft {
            event_id: publication.history_event_id(),
        })
    );
    Ok(())
}
