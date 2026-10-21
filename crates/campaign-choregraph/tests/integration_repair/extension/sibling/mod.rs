//! A branch moved onto a sibling fork one event deeper keeps the generation arithmetic but does
//! not extend the previously validated projection, so it never advances.

/// Admits every publication's draft into `graph` (content-addressed, so identities agree).
fn admit_all(
    mut graph: choregraph_history::HistoryGraph,
    publications: &[&campaign_choregraph::ControlPublication],
) -> Result<choregraph_history::HistoryGraph, String> {
    for publication in publications {
        let admission = crate::suite::identity::observed(graph.admit(publication.draft().clone()))?;
        assert_eq!(admission.event, publication.history_event_id());
        graph = admission.graph;
    }
    Ok(graph)
}

/// The round-2 F1 shape: an accepted stop at the head's child, and the branch moved from the stop
/// onto a stale writer's selection and its result over the pre-stop head, one event deeper.
pub(super) struct ForkOverStop {
    /// Projection of the pre-stop head.
    pub(super) before_view: campaign_choregraph::ValidatedControlProjection,
    /// History and projection of the accepted stop.
    pub(super) stopped: crate::suite::history::Fixture,
    pub(super) stopped_view: campaign_choregraph::ValidatedControlProjection,
    /// The stale writer's lineage, and the stopped branch moved onto it.
    pub(super) forked: crate::suite::history::Fixture,
    pub(super) jumped: crate::suite::history::Fixture,
}

pub(super) fn fork_over_stop(label: &[u8]) -> Result<ForkOverStop, String> {
    let history = crate::suite::candidate::selectable()?;
    let before_stop = history.clone();
    let before_view = before_stop.current()?;
    let stop = crate::suite::integrity::consumer_stop(before_view.state(), label)?;
    let (stopped, _stop_publication) = history.append(stop)?;
    let stopped_view = stopped.current()?;
    assert!(stopped_view.state().stopped.is_some());

    // A stale writer's selection at the pre-stop head, and its result: one event deeper.
    let selection = crate::suite::selection::select_from_current(&before_view)?;
    let (forked, selection_publication) =
        before_stop.append(crate::suite::selection::event(&selection))?;
    let forked_view = forked.current()?;
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        forked_view.state(),
        crate::suite::selection::result_for(&forked_view, &selection),
    ))?;
    let (forked, result_publication) = forked.append(result)?;
    assert_eq!(
        stopped.branch.generation.checked_add(1),
        Some(forked.branch.generation)
    );
    let graph = admit_all(
        stopped.graph.clone(),
        &[&selection_publication, &result_publication],
    )?;
    let jumped = super::moved_to(
        &stopped,
        graph,
        result_publication.history_event_id(),
        forked.payloads.clone(),
    )?;
    Ok(ForkOverStop {
        before_view,
        stopped,
        stopped_view,
        forked,
        jumped,
    })
}

#[test]
// r[verify campaign.history_projection.stale]
fn a_fork_jump_after_an_accepted_stop_does_not_advance() -> Result<(), String> {
    let ForkOverStop {
        stopped,
        stopped_view,
        forked,
        jumped,
        ..
    } = fork_over_stop(b"fork-terminal-stop")?;
    assert_eq!(
        jumped.advance(&stopped_view),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let after_fork = crate::suite::integrity::consumer_stop(
        forked.current()?.state(),
        b"fork-after-result-stop",
    )?;
    assert_eq!(
        jumped.publish_from(&stopped_view, after_fork),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert_eq!(stopped.advance(&stopped_view), Ok(stopped_view.clone()));
    assert!(stopped_view.state().stopped.is_some());
    Ok(())
}

#[test]
fn a_fork_jump_replacing_a_settlement_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let selection = crate::suite::selection::select_from_current(&history.current()?)?;
    let (selected, _p) = history.append(crate::suite::selection::event(&selection))?;
    let selected_view = selected.current()?;
    let first = crate::suite::identity::observed(campaign_core::admit_result(
        selected_view.state(),
        crate::suite::selection::result_for(&selected_view, &selection),
    ))?;
    let (settled, _p) = selected.clone().append(first)?;
    let settled_view = settled.current()?;
    assert!(settled_view.state().outstanding.is_empty());

    let mut different = crate::suite::selection::result_for(&selected_view, &selection);
    different.payload_id =
        crate::suite::identity::derive(campaign_core::IdKind::EventPayload, b"fork-retry-payload");
    let second = crate::suite::identity::observed(campaign_core::admit_result(
        selected_view.state(),
        different,
    ))?;
    let (forked, second_publication) = selected.append(second)?;
    let stop =
        crate::suite::integrity::consumer_stop(forked.current()?.state(), b"fork-settled-stop")?;
    let (forked, stop_publication) = forked.append(stop)?;
    assert_eq!(
        settled.branch.generation.checked_add(1),
        Some(forked.branch.generation)
    );
    let graph = admit_all(
        settled.graph.clone(),
        &[&second_publication, &stop_publication],
    )?;
    let jumped = super::moved_to(
        &settled,
        graph,
        stop_publication.history_event_id(),
        forked.payloads.clone(),
    )?;

    assert_eq!(
        jumped.advance(&settled_view),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let honest_stop =
        crate::suite::integrity::consumer_stop(settled_view.state(), b"fork-settled-honest-stop")?;
    settled
        .publish_from(&settled_view, honest_stop.clone())
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        jumped.publish_from(&settled_view, honest_stop),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert_eq!(settled.advance(&settled_view), Ok(settled_view.clone()));
    Ok(())
}
