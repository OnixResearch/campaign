//! What a control graph holds under the projection's envelope besides the control lineage.
//!
//! One Campaign campaign per graph and envelope: the adapter bounds one campaign's control
//! lineage, its outstanding settlements, and its stop against the graph's event limit, so a second
//! campaign's control root under the same projection schema, adapter, and control domain would
//! book the same graph room twice. Root admission refuses a root on a graph that already holds a
//! Campaign control event under the envelope, and validation refuses a graph holding another
//! control root, both with `SharedControlGraph`. Campaigns under other envelopes are other
//! writers of the graph, bound by the exposed room like every other writer.
//!
//! Every Campaign control draft under the envelope that descends from the branch head through
//! control-parent links (a pending draft, racing siblings, and the drafts a pipelining host
//! chained below another draft) is part of the reserve, found in one parent-to-children pass over
//! the graph. The draft republished first is the child of the head first in Choregraph
//! event-identity order; the drafts chained below it become children of the head, in chain
//! order, as each republication commits. Hosts publish only on durably committed branch values,
//! and publication chains a draft below another only for a host that violated that obligation;
//! counting chained drafts is a conservative defence, not a recovery guarantee (see
//! [`crate::headroom::reserve`]).

/// The Campaign control drafts pending on a branch head.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Pending {
    /// The draft to republish first: the child of the head first in Choregraph event-identity
    /// order.
    pub(crate) first: Option<choregraph_history::EventId>,
    /// Every Campaign control draft descending from the head, in Choregraph event-identity order.
    pub(crate) drafts: alloc::vec::Vec<choregraph_history::EventId>,
}

/// Campaign control events of `graph` under `binding`'s envelope with one control-parent link,
/// grouped by parent; each group in Choregraph event-identity order.
type Children = alloc::collections::BTreeMap<
    choregraph_history::EventId,
    alloc::vec::Vec<choregraph_history::EventId>,
>;

/// Refuses admitting the control root `root` to `graph` while the graph holds another Campaign
/// control event under the admission's envelope (`envelope`, the admission's projection binding).
///
/// # Errors
///
/// Returns `SharedControlGraph` naming the first such event in Choregraph event-identity order.
// r[impl campaign.initial_admission.one_per_graph]
pub(crate) fn check_root_admission(
    graph: &choregraph_history::HistoryGraph,
    envelope: &campaign_core::ProjectionBinding,
    root: choregraph_history::EventId,
) -> crate::contracts::AdapterResult<()> {
    let occupied = graph.events().find(|(event_id, event)| {
        **event_id != root && crate::envelope::is_control_event(event.content(), envelope)
    });
    match occupied {
        Some((event_id, _)) => Err(crate::contracts::AdapterError::SharedControlGraph {
            event_id: *event_id,
        }),
        None => Ok(()),
    }
}

/// Scans `graph` once for the control lineage rooted at `root` and headed at `head`: refuses
/// another Campaign control root under `binding`'s envelope and returns the drafts pending on
/// `head`.
///
/// # Errors
///
/// Returns `SharedControlGraph` naming the first other control root in Choregraph
/// event-identity order.
// r[impl campaign.publication_fence.pending]
// r[impl campaign.publication_fence.pending_several]
// r[impl campaign.publication_fence.pending_blocked]
// r[impl campaign.publication_fence.pending_reserve]
// r[impl campaign.initial_admission.one_per_graph]
pub(crate) fn scan(
    graph: &choregraph_history::HistoryGraph,
    binding: &campaign_core::ProjectionBinding,
    root: choregraph_history::EventId,
    head: choregraph_history::EventId,
) -> crate::contracts::AdapterResult<Pending> {
    let mut children = Children::new();
    for (event_id, event) in graph.events() {
        let content = event.content();
        if *event_id != root && crate::envelope::is_control_root(content, binding) {
            return Err(crate::contracts::AdapterError::SharedControlGraph {
                event_id: *event_id,
            });
        }
        if let Some(parent) = crate::envelope::control_parent(content, binding) {
            children.entry(parent).or_default().push(*event_id);
        }
    }
    let first = children
        .get(&head)
        .and_then(|drafts| drafts.first().copied());
    let drafts = descendants(&children, head);
    debug_assert_eq!(first.is_some(), !drafts.is_empty());
    Ok(Pending { first, drafts })
}

/// The number of Campaign control drafts under `binding`'s envelope that `graph` holds chained
/// below `event` (its children, their children, and so on), each of which republication counts
/// as a selection: exact for a chained selection and conservative for any other chained draft
/// (the pipelined-chain residual of [`crate::headroom::reserve`]).
// r[impl campaign.publication_fence.pending_reserve]
// r[impl campaign.publication_fence.durable_base]
pub(crate) fn chained_below(
    graph: &choregraph_history::HistoryGraph,
    binding: &campaign_core::ProjectionBinding,
    event: choregraph_history::EventId,
) -> usize {
    let mut children = Children::new();
    for (event_id, history_event) in graph.events() {
        if let Some(parent) = crate::envelope::control_parent(history_event.content(), binding) {
            children.entry(parent).or_default().push(*event_id);
        }
    }
    descendants(&children, event).len()
}

/// Every event below `from` in `children`, in Choregraph event-identity order, found level by
/// level. History events are content-addressed over their parents, so the parent links form a
/// forest: every level below `from` expands parents no earlier level expanded, and the walk ends
/// within one level per parent group.
fn descendants(
    children: &Children,
    from: choregraph_history::EventId,
) -> alloc::vec::Vec<choregraph_history::EventId> {
    let mut found = alloc::vec::Vec::new();
    let mut level = alloc::vec![from];
    for _ in 0..children.len() {
        let below: alloc::vec::Vec<_> = level
            .iter()
            .filter_map(|parent| children.get(parent))
            .flatten()
            .copied()
            .collect();
        if below.is_empty() {
            break;
        }
        found.extend_from_slice(&below);
        level = below;
    }
    found.sort_unstable();
    debug_assert!(found.is_sorted());
    found
}
