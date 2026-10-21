//! History shapes presented through every selected state: the adapter's load and cache paths
//! hold Campaign's projection of the state (on a graph that grew by one exploration moment after
//! its head) and meet the fast-forward to its next event when one exists (a consumer stop of a
//! running state, the post-stop settlement of a stopped state with outstanding selections),
//! every rewind to a shorter prefix, its own lineage on the graph before that moment (a stale
//! graph), every forged and suffix catalog class its head admits, and forks of the history before
//! its head (`forks`): at depths zero and one with every head kind the fork state admits, and
//! greedily at every depth value and head sub-kind some 3-way combination of the state still
//! lacks; a bare admission, which has no earlier history, meets another campaign's root instead
//! of forks and classes. Every answer must equal Campaign's.

mod forks;
mod shapes;

/// Exploration moment the selected state's graph admits after its head.
const LATER_MOMENT: crate::vocabulary::MomentHandle = crate::vocabulary::MomentHandle(90);

/// Presents every shape through the replayed state (of `class`) to the load and cache paths;
/// records in `pairs` the fork values the state admits and those presented through it.
///
/// # Errors
///
/// Returns the error of a construction and the oracle mismatch of a presentation.
// r[impl campaign.conformance.negative]
// r[impl campaign.conformance.state_coverage]
// r[impl campaign.history_projection.stale]
// r[impl campaign.cache.invalid]
pub(super) fn present(
    session: &mut crate::harness::Session<'_, '_>,
    (replayed, class): (
        &super::recipes::replay::Replayed,
        super::features::StateClass,
    ),
    pairs: &mut super::forked::Pairs,
) -> crate::verdicts::ConformanceResult<()> {
    let captured = session.history();
    session.add_moment(crate::vocabulary::ScenarioMoment {
        handle: LATER_MOMENT,
        parent: Some(crate::harness::history::ROOT),
    })?;
    session.hold_current()?;
    let view = session
        .presented_projection(crate::harness::oracle::Presentation::ThroughHeld)?
        .cloned()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the session holds the selected state's projection",
        })?;
    let history = session.history();
    let presentations = shapes::every(
        session,
        (replayed, class, pairs),
        (&history, &view),
        captured,
    )?;
    let admitted = presentations
        .iter()
        .fold(0, |admitted, (_, values)| admitted | values);
    pairs.admit(class, admitted);
    let anchors: alloc::vec::Vec<&campaign_choregraph::ValidatedControlProjection> =
        core::iter::once(&view)
            .chain(replayed.before.as_ref().map(|base| &base.view))
            .collect();
    for ((entry, answer), values) in &presentations {
        let host = crate::harness::oracle::hosted::Host {
            maximum_events: entry.maximum_events,
            previous: Some(&view),
        };
        super::super::histories::presented::present_load(session, entry, host, answer)?;
        let states =
            super::super::histories::cached::states(session, entry, view.state(), &anchors);
        super::super::histories::presented::present_caches(session, entry, host, answer, &states)?;
        pairs.present(class, *values);
    }
    debug_assert!(!presentations.is_empty());
    Ok(())
}
