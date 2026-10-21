//! The states whose caches the cache oracle presents for one catalog entry: the state the
//! entry's payloads replay to (when they replay), the current state, and the held state.

/// The state the entry's payloads replay to: the entry's own when its constructor knows it,
/// otherwise the longest of `anchors` whose control events are a prefix of the payloads advanced
/// by the remaining events (folding `apply_control_event` over them equals `reconstruct` of the
/// whole lineage), otherwise `reconstruct`; `None` when the payloads do not replay.
fn replayed(
    entry: &super::Entry,
    anchors: &[&campaign_choregraph::ValidatedControlProjection],
) -> Option<campaign_core::FrontierState> {
    if let Some(known) = &entry.replayed {
        return Some(known.clone());
    }
    let events: alloc::vec::Vec<campaign_core::CampaignControlEvent> = entry
        .history
        .payloads
        .iter()
        .map(|payload| payload.control_event.clone())
        .collect();
    let anchor = anchors
        .iter()
        .filter(|anchor| events.starts_with(anchor.control_events()))
        .max_by_key(|anchor| anchor.control_events().len());
    let Some(anchor) = anchor else {
        return campaign_core::reconstruct(&events).ok();
    };
    let rest = events
        .get(anchor.control_events().len()..)
        .unwrap_or_default();
    debug_assert!(events.starts_with(anchor.control_events()));
    debug_assert_eq!(
        rest.len().saturating_add(anchor.control_events().len()),
        events.len()
    );
    rest.iter()
        .try_fold(anchor.state().clone(), |state, event| {
            campaign_core::apply_control_event(state, event)
        })
        .ok()
}

/// The states whose caches one entry presents: the state the history's payloads replay to (when
/// they replay), the current state, and the `held` state, each once; `anchors` are projections
/// whose lineages the entry may extend.
pub(in crate::catalog::evidence) fn states(
    session: &crate::harness::Session<'_, '_>,
    entry: &super::Entry,
    held: &campaign_core::FrontierState,
    anchors: &[&campaign_choregraph::ValidatedControlProjection],
) -> alloc::vec::Vec<campaign_core::FrontierState> {
    let mut states = alloc::vec::Vec::with_capacity(3);
    if let Some(claimed) = replayed(entry, anchors) {
        states.push(claimed);
    }
    states.extend([session.state.clone(), held.clone()]);
    let mut unique: alloc::vec::Vec<campaign_core::FrontierState> =
        alloc::vec::Vec::with_capacity(states.len());
    for state in states {
        if !unique.iter().any(|known| known.state_id == state.state_id) {
            unique.push(state);
        }
    }
    debug_assert!(!unique.is_empty());
    unique
}
