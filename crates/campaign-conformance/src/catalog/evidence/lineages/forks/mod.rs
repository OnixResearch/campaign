//! Forks of a held head: a branch moved from the current head onto a writer's fork of the history
//! one event before a dropped head. A fork publishes fill events over that history and ends in a
//! head (`shaped::Head`): one of the catalog's six head kinds (`heads`: a consumer stop, a
//! guidance event, a candidate batch, a selection, a result, or an unobserved closure) or one of
//! the state coverage's event sub-kinds (`subkinds`); a settlement head over a state with nothing
//! outstanding publishes a ranked selection as its last fill event. The catalog presents forks at
//! the dropped head's own depth, one to `MAX_FORK_DEPTH` events beyond it, and at the deepest
//! depth the presented projection bound, the control-event room, the graph room, and the fill's
//! room admit (`maximal_depth`), so a shell that resyncs once the presented lineage is some number
//! of events ahead of a held projection, or unless the presented head is of some kind, meets a
//! fork it must refuse. Each fork is valid history on its own and drops its dropped head; a head
//! that would reproduce the dropped head is not a fork and is not presented.

pub(in crate::catalog::evidence) mod extent;
pub(in crate::catalog::evidence) mod fill;
pub(in crate::catalog::evidence) mod heads;
mod points;
pub(in crate::catalog::evidence) mod shaped;
pub(in crate::catalog::evidence) mod subkinds;

/// One history a writer forks: the history, Campaign's projection of it, and its state.
pub(in crate::catalog::evidence) struct Base {
    pub(in crate::catalog::evidence) history: crate::harness::oracle::HistoryView,
    pub(in crate::catalog::evidence) view: campaign_choregraph::ValidatedControlProjection,
    pub(in crate::catalog::evidence) state: campaign_core::FrontierState,
}

/// The histories forks are built over: the earlier held head (H0, one event before the current
/// head), the history one event before H0's head, and the histories before the campaign's
/// candidate batch and its first and second guidance event (each captured before that event's
/// publication).
pub(in crate::catalog::evidence) struct Earlier {
    /// The earlier held head (H0).
    pub(in crate::catalog::evidence) head: Base,
    /// The history before H0's head.
    pub(in crate::catalog::evidence) before: Base,
    /// The histories before the earlier opening events, in lineage order.
    pub(in crate::catalog::evidence) opening: alloc::vec::Vec<crate::harness::oracle::HistoryView>,
}

/// One publication over a presented history: the published history, Campaign's projection of
/// it, and the published draft.
pub(in crate::catalog::evidence) struct Published {
    pub(in crate::catalog::evidence) history: crate::harness::oracle::HistoryView,
    pub(in crate::catalog::evidence) view: campaign_choregraph::ValidatedControlProjection,
    draft: campaign_choregraph::EventDraft,
}

/// Publishes `event` over `base` as a fast-forward of `view`.
pub(in crate::catalog::evidence) fn publish_over(
    session: &crate::harness::Session<'_, '_>,
    base: &crate::harness::oracle::HistoryView,
    view: &campaign_choregraph::ValidatedControlProjection,
    event: campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<Published> {
    let publication = campaign_choregraph::plan_control_event_publication(
        campaign_choregraph::ControlPublicationRequest {
            current: super::projection_request(session, base)?,
            previous: view,
            control_event: event,
        },
    )?;
    let mut payloads = base.payloads.clone();
    payloads.push(campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: publication.history_event_id(),
        control_event: publication.control_event().clone(),
    });
    let history = crate::harness::oracle::HistoryView {
        graph: publication.graph_after_admission().clone(),
        branch: publication.branch_plan().after.clone(),
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads,
    };
    let view = campaign_choregraph::validate_control_advance(
        view,
        &super::projection_request(session, &history)?,
    )?;
    debug_assert_eq!(view.control_events().len(), history.payloads.len());
    debug_assert_eq!(history.branch.head, publication.history_event_id());
    Ok(Published {
        history,
        view,
        draft: publication.draft().clone(),
    })
}

/// `fork` over the session's current graph: the fork's drafts admitted next to the current head.
fn over_current(
    session: &crate::harness::Session<'_, '_>,
    fork: crate::harness::oracle::HistoryView,
    drafts: alloc::vec::Vec<campaign_choregraph::EventDraft>,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let mut graph = session.graph.clone();
    for draft in drafts {
        graph = graph.admit(draft)?.graph;
    }
    debug_assert!(graph.contains(session.branch.head));
    Ok(crate::harness::oracle::HistoryView { graph, ..fork })
}

/// One fork of a dropped head: its depth beyond the dropped head, whether that is the deepest
/// depth the base admits for its head, its head, the history of the branch moved onto it next to
/// the current head, and Campaign's state of that history.
pub(in crate::catalog::evidence) struct Fork<H = heads::HeadKind> {
    pub(in crate::catalog::evidence) depth: usize,
    pub(in crate::catalog::evidence) is_deepest: bool,
    pub(in crate::catalog::evidence) head: H,
    pub(in crate::catalog::evidence) history: crate::harness::oracle::HistoryView,
    pub(in crate::catalog::evidence) state: campaign_core::FrontierState,
}

/// Forks over `base` dropping `dropped` (the head one event after `base`, carrying
/// `dropped_event`): one per presented depth and head kind the fork state admits, admitted next
/// to the current head, the maximal depth only with the `deepest_heads`. A head that must leave
/// room for more control events than a stop (its stop, and a selection's settlement) is presented
/// at most at the maximal depth less that room, its deepest depth; over a stopped base only
/// settlement heads are admissible, and they need no room.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction, and `Diverged` for a
/// fork of another depth or generation than requested or when no fork fits the bounds.
// r[impl campaign.conformance.bounds]
// r[impl campaign.conformance.negative]
pub(in crate::catalog::evidence) fn dropping(
    session: &mut crate::harness::Session<'_, '_>,
    base: &Base,
    dropped: (
        &campaign_choregraph::BranchRef,
        &campaign_core::CampaignControlEvent,
    ),
    (depths, deepest_heads): (&[usize], &[heads::HeadKind]),
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Fork>> {
    let maximal = extent::maximal_depth(session, base)?;
    let is_stopped = base.state.stopped.is_some();
    let mut shapes: alloc::vec::Vec<(usize, heads::HeadKind)> =
        alloc::vec::Vec::with_capacity(depths.len().saturating_mul(heads::HEAD_KINDS.len()));
    for head in heads::HEAD_KINDS {
        let ceiling = maximal.saturating_sub(head.reserve(is_stopped));
        for depth in depths {
            if *depth == maximal && !deepest_heads.contains(&head) {
                continue;
            }
            let shape = ((*depth).min(ceiling), head);
            if !shapes.contains(&shape) {
                shapes.push(shape);
            }
        }
    }
    let presented: alloc::vec::Vec<usize> = shapes.iter().map(|(depth, _)| *depth).collect();
    let forking = shaped::Forking::over(session, base, dropped, &presented)?;
    debug_assert_eq!(forking.maximal(), maximal);
    let mut forks = alloc::vec::Vec::with_capacity(shapes.len());
    for shape in shapes {
        forks.extend(forking.fork(session, shape)?);
    }
    debug_assert!(forks.iter().all(|fork| fork.depth <= maximal));
    Ok(forks)
}
