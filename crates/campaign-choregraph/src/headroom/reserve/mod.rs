//! The control reserve a validated projection records and exposes to hosts.
//!
//! A validated projection's control history still owes one settlement per outstanding selection
//! and the stop of an open campaign. Every Campaign control draft under the envelope that
//! descends from the branch head (a publication admitted to the graph whose branch update has not
//! committed, racing siblings, and drafts chained below another draft; see [`crate::occupancy`])
//! is part of that reserve: its graph event is already held, and once republished it adds its own
//! lineage event and, if it publishes a selection, that selection's settlement. A draft's control
//! event cannot be read from the graph (only its identity and payload digest are), so each
//! pending draft is counted as a selection, the largest demand one control event makes.
//!
//! Hosts publish a control event only on a durably committed branch value: they never plan or
//! publish through a publication's planned branch value before its compare-and-set is durable (no
//! pipelining). For such a host every draft below the head is a child of the head (its own
//! publication whose compare-and-set did not commit, or siblings racing publishers left), and
//! republishing one checks that draft's actual demand, which never exceeds this count. A
//! non-pipelining host that appends only within
//! [`crate::ValidatedControlProjection::graph_room`] therefore never strands a pending
//! publication, an outstanding settlement, or the stop; of racing siblings whose joint demand
//! exceeds the host capacity, only a draft whose actual demand still fits republishes.
//!
//! Publication chains drafts below another draft only for a host that pipelined. The reserve
//! counts each of them as a selection, as a conservative defence, and republishing a draft
//! checks its actual demand plus every draft chained below it again counted as a selection. That
//! count is exact for a chained selection, one event over for a chained draft of other new work,
//! and two events over for a chained settlement or stop, whose event the graph already holds and
//! whose slot (an outstanding settlement or the stop) is still counted. A pipelined chain can
//! therefore be refused at a bound (`ProjectionCapacityExhausted`, `GenerationCapacityExhausted`,
//! or `GraphCapacityExhausted`) although its drafts need no new graph event; its head then stays
//! blocked with `PendingControlDraft`. This pipelined-chain residual is outside the recovery
//! guarantee. Planning through the projection starts from the same demand.

/// The reserve a validated projection records: the events its control history still owes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Recorded {
    /// Control-lineage events owed beyond the lineage: each pending draft's own event, one
    /// settlement per outstanding selection and per pending draft, and the stop of an open
    /// campaign.
    pub(crate) control_events: u32,
    /// Graph events owed beyond the validated graph: the control events less the pending drafts'
    /// own events, which the graph already holds.
    pub(crate) graph_events: u32,
}

/// The demand of a control history of `extent` (its control lineage and the graph holding it),
/// reconstructed to `state`, with `drafts` Campaign control drafts the graph already holds still
/// to be published after it: each draft counted as a published selection, one settlement per
/// outstanding selection, and the stop of an open campaign. Counts saturate, so an
/// unrepresentable demand fits no bound.
pub(super) fn demand(
    extent: super::limits::Extent,
    state: &campaign_core::FrontierState,
    drafts: usize,
) -> super::limits::Demand {
    let demand = super::limits::Demand {
        extent: super::limits::Extent {
            lineage: extent.lineage.saturating_add(drafts),
            graph: extent.graph,
        },
        outstanding: state.outstanding.len().saturating_add(drafts),
        open: state.stopped.is_none(),
    };
    debug_assert!(demand.extent.lineage >= extent.lineage);
    demand
}

/// The demand of the validated projection `current` (see [`demand`]).
// r[impl campaign.publication_fence.pending_reserve]
pub(super) fn held(
    current: &crate::contracts::ValidatedControlProjection,
) -> super::limits::Demand {
    demand(
        super::limits::Extent {
            lineage: current.control_events.len(),
            graph: current.graph_events,
        },
        &current.state,
        current.pending.drafts.len(),
    )
}

/// The reserve a projection validated with the control lineage and graph of `extent`,
/// reconstructed to `state` with `drafts` Campaign control drafts pending below its head,
/// records.
///
/// # Errors
///
/// Returns `Campaign(ArithmeticOverflow)` for a reserve beyond `u32::MAX` events.
// r[impl campaign.publication_fence.pending_reserve]
// r[impl campaign.publication_fence.durable_base]
pub(crate) fn record(
    extent: super::limits::Extent,
    state: &campaign_core::FrontierState,
    drafts: usize,
) -> crate::contracts::AdapterResult<Recorded> {
    let graph_events = u32::try_from(demand(extent, state, drafts).reserve())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let control_events = u32::try_from(drafts)
        .ok()
        .and_then(|drafts| graph_events.checked_add(drafts))
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    debug_assert!(control_events >= graph_events);
    Ok(Recorded {
        control_events,
        graph_events,
    })
}
