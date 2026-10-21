//! The validated view of one Choregraph control projection and the reserve it exposes to hosts.

/// Validated view of one Choregraph projection and its Campaign state.
///
/// Only [`crate::validate_control_projection`] and [`crate::validate_control_advance`] construct
/// this value, so holding one proves the state was reconstructed from exactly the bound
/// fast-forward control lineage:
///
/// ```compile_fail
/// fn forge(
///     binding: campaign_core::ProjectionBinding,
///     identity: campaign_choregraph::Digest,
///     state: campaign_core::FrontierState,
/// ) -> campaign_choregraph::ValidatedControlProjection {
///     campaign_choregraph::ValidatedControlProjection {
///         binding,
///         choregraph_projection_identity: identity,
///         control_events: Vec::new(),
///         state,
///     }
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedControlProjection {
    pub(crate) binding: campaign_core::ProjectionBinding,
    pub(crate) choregraph_projection_identity: choregraph_history::Digest,
    pub(crate) control_events: alloc::vec::Vec<campaign_core::CampaignControlEvent>,
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) maximum_events: u32,
    pub(crate) graph_events: u32,
    pub(crate) graph_max_events: u32,
    pub(crate) pending: crate::occupancy::Pending,
    pub(crate) reserve: crate::headroom::reserve::Recorded,
}

impl ValidatedControlProjection {
    /// Campaign binding derived from the supplied Choregraph graph and branch.
    #[must_use]
    pub const fn binding(&self) -> &campaign_core::ProjectionBinding {
        &self.binding
    }

    /// Exact Choregraph linear projection identity.
    #[must_use]
    pub const fn choregraph_projection_identity(&self) -> choregraph_history::Digest {
        self.choregraph_projection_identity
    }

    /// Campaign control events in the accepted linear order.
    #[must_use]
    pub const fn control_events(&self) -> &[campaign_core::CampaignControlEvent] {
        self.control_events.as_slice()
    }

    /// Frontier state reconstructed by `campaign-core`.
    #[must_use]
    pub const fn state(&self) -> &campaign_core::FrontierState {
        &self.state
    }

    /// Host Choregraph projection bound the projection was validated under; planning through it
    /// keeps the same control capacity publication enforces.
    #[must_use]
    pub const fn maximum_events(&self) -> u32 {
        self.maximum_events
    }

    /// Events the Choregraph graph held when the projection was validated, control and other
    /// events alike; planning through it reserves graph capacity as publication does, and a later
    /// fast-forward must present a graph holding at least these events.
    #[must_use]
    pub const fn graph_events(&self) -> u32 {
        self.graph_events
    }

    /// The Choregraph graph's own event limit (`HistoryLimits::max_events`) the projection was
    /// validated under.
    #[must_use]
    pub const fn graph_max_events(&self) -> u32 {
        self.graph_max_events
    }

    /// Control-lineage events this campaign still owes beyond its validated lineage: one
    /// settlement per outstanding selection, the stop of an open campaign, and for every Campaign
    /// control draft under the envelope that descends from the branch head (the pending draft,
    /// racing siblings, and drafts a pipelining host chained below another draft) its own lineage
    /// event and one settlement, each draft counted as a selection (the largest demand one control
    /// event makes).
    ///
    /// Publication and planning add this count to the lineage when they check the projection
    /// bound and the branch generation range. It is a count, not a promise: when other writers
    /// consumed the room outside Campaign publication, the lineage plus this reserve may already
    /// exceed a bound, and then every publication that cannot shrink it is refused. For a chained
    /// draft that settles a selection or records the stop the count is conservative (see
    /// [`Self::graph_room`]).
    #[must_use]
    // r[impl campaign.publication_fence.pending_reserve]
    pub const fn reserved_control_events(&self) -> u32 {
        self.reserve.control_events
    }

    /// Graph events this campaign still owes beyond [`Self::graph_events`]:
    /// [`Self::reserved_control_events`] less the pending drafts' own events, which the graph
    /// already holds (one settlement per outstanding selection and per pending draft, and the
    /// stop of an open campaign).
    #[must_use]
    // r[impl campaign.publication_fence.pending_reserve]
    pub const fn reserved_graph_events(&self) -> u32 {
        self.reserve.graph_events
    }

    /// Graph events writers other than this campaign's control lineage may still append to the
    /// validated graph: [`Self::graph_max_events`] less [`Self::graph_events`] and
    /// [`Self::reserved_graph_events`], or zero once the reserve is consumed.
    ///
    /// Every other append counts against it: exploration moments, reported children, and events
    /// of campaigns under other envelopes (a graph holds one Campaign campaign per envelope).
    /// This campaign's own publications are checked against the whole reserve instead: a
    /// settlement or the stop draws on the reserve and leaves the room unchanged, while new work
    /// (candidates, guidance, retirements, prunes) takes one event of the room and a new
    /// selection two (its event and its settlement).
    ///
    /// The recovery guarantee holds for hosts that publish only on durably committed branch
    /// values (never planning or publishing through a publication's planned branch value before
    /// its compare-and-set is durable): such a host that keeps the other appends within this room
    /// can republish its pending draft (a publication whose compare-and-set did not commit, or one
    /// of the siblings racing publishers left, of which only a draft whose actual demand still
    /// fits republishes when their joint demand exceeds the capacity), settle every outstanding
    /// selection, and publish the stop. A pipelining host that chained drafts below another
    /// draft is outside the guarantee: the reserve counts every chained draft as a selection as a
    /// defence, but republishing the draft above counts each chained draft as a selection again,
    /// two events over for a chained settlement or stop, so the chain may be refused at the bound
    /// with a capacity error and block the head with `PendingControlDraft` although no other
    /// writer appended (see [`crate::plan_control_event_publication`]).
    #[must_use]
    // r[impl campaign.publication_fence.pending_reserve]
    // r[impl campaign.publication_fence.durable_base]
    pub const fn graph_room(&self) -> u32 {
        self.graph_max_events
            .saturating_sub(self.graph_events)
            .saturating_sub(self.reserve.graph_events)
    }
}
