//! How deep the catalog forks a base: the deepest fork the bounds admit and the presented
//! depths.

/// Largest named depth of a generated fork beyond the dropped head; the catalog presents every
/// depth from one to this bound and the maximal depth.
const MAX_FORK_DEPTH: usize = 3;

/// The deepest fork over `base` beyond the head one event after it. After a stop only the
/// settlements of the selections still outstanding are admissible, so a fork over a stopped base
/// settles all but one in its fill and the last in its head. Otherwise the fork's lineage (the
/// base history, its fill events, and its head) fills `HISTORY_MAX_EVENTS` less the settlement
/// reserve of the base's outstanding selections, unless Campaign's control-event bound, the
/// session graph's event bound, or the fill's own room (`fill::room`) admits fewer fill events.
/// The fill continues with coalesced duplicates once its record room is used, so a record bound
/// limits the depth only while no retained candidate is or can be guided.
///
/// # Errors
///
/// Returns `Diverged` when no fork of a running base deeper than the dropped head fits the
/// bounds, or a stopped base holds nothing outstanding.
// r[impl campaign.conformance.bounds]
pub(in crate::catalog::evidence) fn maximal_depth(
    session: &crate::harness::Session<'_, '_>,
    base: &super::Base,
) -> crate::verdicts::ConformanceResult<usize> {
    let outstanding = base.state.outstanding.len();
    if base.state.stopped.is_some() {
        return outstanding
            .checked_sub(1)
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "a stopped fork base holds an outstanding selection",
            });
    }
    let bounds = &base.state.admission.bounds;
    let dropped = base
        .history
        .payloads
        .len()
        .checked_add(1)
        .and_then(|dropped| dropped.checked_add(outstanding));
    let by_projection = usize::try_from(crate::harness::HISTORY_MAX_EVENTS)
        .ok()
        .zip(dropped)
        .and_then(|(bound, dropped)| bound.checked_sub(dropped));
    let reserve = u64::try_from(outstanding)
        .ok()
        .and_then(|outstanding| outstanding.checked_add(1));
    let by_events = reserve
        .and_then(|reserve| {
            bounds
                .max_control_events
                .checked_sub(base.state.control_event_count)
                .and_then(|room| room.checked_sub(reserve))
        })
        .and_then(|room| usize::try_from(room).ok());
    let by_graph = session
        .graph
        .limits()
        .max_events
        .checked_sub(session.graph.len())
        .and_then(|room| room.checked_sub(1))
        .and_then(|room| usize::try_from(room).ok())
        .and_then(|room| room.checked_sub(outstanding));
    let by_fill = Some(super::fill::room(&base.state));
    let depth = [by_projection, by_events, by_graph, by_fill]
        .into_iter()
        .try_fold(usize::MAX, |depth, room| room.map(|room| depth.min(room)))
        .filter(|depth| *depth > 0)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the bounds admit a fork deeper than the dropped head",
        })?;
    debug_assert!(by_projection.is_none_or(|room| depth <= room));
    Ok(depth)
}

/// The depths the catalog presents for forks over `base`: the dropped head's own depth (zero),
/// every named depth from one to `MAX_FORK_DEPTH` below the maximal depth, then the maximal
/// depth (zero over a stopped base with one selection outstanding).
///
/// # Errors
///
/// Returns `Diverged` when no fork deeper than the dropped head fits the bounds.
pub(in crate::catalog::evidence) fn depths(
    session: &crate::harness::Session<'_, '_>,
    base: &super::Base,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<usize>> {
    let maximal = maximal_depth(session, base)?;
    let mut depths: alloc::vec::Vec<usize> = (0..=MAX_FORK_DEPTH)
        .filter(|depth| *depth < maximal)
        .collect();
    depths.push(maximal);
    debug_assert!(depths.last() == Some(&maximal));
    debug_assert!(depths.len() <= MAX_FORK_DEPTH.saturating_add(2));
    Ok(depths)
}
