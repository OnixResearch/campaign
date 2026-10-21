//! One pending control draft per branch head.
//!
//! Choregraph admits a publication's draft to the graph before the host's branch compare-and-set,
//! so a draft whose parent is the branch head is a publication whose branch update is pending (a
//! host that crashed after admitting the draft, or the winner of a compare-and-set race still
//! committing). Its event already consumes the graph capacity the control lineage reserves, so no
//! other event may be planned or published on that head: a CAS loser or a stale publisher would
//! otherwise admit a second draft that consumes the winning lineage's reserve. The pending draft
//! is completed by republishing its identical control event (idempotent republication), which
//! counts the held draft once. Hosts serialize graph appends per control branch: they republish
//! pending drafts before appending any other graph event, and keep other appends within the room
//! the validated projection exposes, whose reserve counts every Campaign draft pending below the
//! head ([`crate::ValidatedControlProjection::graph_room`], [`crate::occupancy`]).
//!
//! Hosts publish a control event only on a durably committed branch value: they never plan or
//! publish through a publication's planned branch value (its branch plan's `after`) before that
//! compare-and-set is durable. A pipelining host that violates this and publishes a second event
//! on the planned branch value of the first leaves a chain of drafts below the head. The chain is
//! completed in order where it still fits: the child of the head is pending first, and each draft
//! below it becomes the pending draft once the draft above it commits. A chained draft's control
//! event presented from the head before the draft above it commits is refused: a selection with
//! `PublicationFenceMismatch`, and every other control event with the Campaign replay error
//! `StaleState`, because it names the state after the draft above it as its state before (only a
//! hand-built chained draft whose event names the head's own state reaches `PendingControlDraft`,
//! as its draft is not the one publication builds). Republishing the draft above counts every
//! chained draft as a selection, which over-counts a chained settlement or stop, so a pipelined
//! chain may be refused at a bound and block the head (the pipelined-chain residual of
//! [`crate::headroom::reserve`]).
//!
//! Several control drafts pending on one head (racing publishers from the same held projection,
//! which the serialization rule forbids) are named deterministically: the planner, every refused
//! publication, and [`crate::pending_control_draft`] name the first of them in Choregraph
//! event-identity order. Republishing any one of them exactly stays allowed; once its branch
//! update commits, the others are no longer below the head and leave the reserve, while their
//! events stay in the graph like any other writer's.
//!
//! There is no abandonment: a draft the host cannot complete (a forged or foreign child under the
//! Campaign envelope whose payload is not its named control event, a lost journal, or a lowered
//! bound) blocks the head, and every plan and publication on it is refused with
//! `PendingControlDraft` naming that draft. This residual relies on hosts serializing graph
//! appends per control branch.

/// A Campaign control draft pending on a control branch head, decoded from its Choregraph event.
///
/// Only [`crate::pending_control_draft`] constructs this value, so it always names a draft under
/// the projection's envelope identities whose only parent is the validated branch head:
///
/// ```compile_fail
/// fn forge(
///     history_event: campaign_choregraph::EventId,
///     control_event: campaign_core::Id,
///     payload: campaign_core::Id,
/// ) -> campaign_choregraph::PendingDraft {
///     campaign_choregraph::PendingDraft {
///         history_event,
///         control_event,
///         payload,
///     }
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
// r[impl campaign.publication_fence.pending_recovery]
pub struct PendingDraft {
    pub(crate) history_event: choregraph_history::EventId,
    pub(crate) control_event: campaign_core::Id,
    pub(crate) payload: campaign_core::Id,
}

impl PendingDraft {
    /// The pending draft's Choregraph event identity.
    #[must_use]
    pub const fn history_event_id(&self) -> choregraph_history::EventId {
        self.history_event
    }

    /// The Campaign control-event identity the draft's metadata names, by which the host finds the
    /// control event it republishes.
    #[must_use]
    pub const fn control_event_id(&self) -> campaign_core::Id {
        self.control_event
    }

    /// The Campaign control payload identity the draft carries.
    #[must_use]
    pub const fn payload_id(&self) -> campaign_core::Id {
        self.payload
    }
}

/// Refuses planning or publishing another event while the control draft `pending` (recorded by
/// the projection's validation) is pending on its branch head.
///
/// # Errors
///
/// Returns `PendingControlDraft` naming the pending draft.
// r[impl campaign.publication_fence.pending]
// r[impl campaign.publication_fence.pending_several]
// r[impl campaign.publication_fence.pending_blocked]
pub(crate) const fn check_none(
    pending: Option<choregraph_history::EventId>,
) -> crate::contracts::AdapterResult<()> {
    match pending {
        Some(event_id) => Err(crate::contracts::AdapterError::PendingControlDraft { event_id }),
        None => Ok(()),
    }
}

/// Decodes the Campaign control draft pending on the current branch head, validated against the
/// held projection, so a recovering host can republish its control event.
///
/// The current inputs are validated as a fast-forward of `previous` (as publication validates
/// them). The pending draft is a Campaign control draft under the projection's schema, adapter,
/// and control-domain identities whose only parent is the current branch head, the first in
/// Choregraph event-identity order when several are pending (the one the planner's
/// `PendingControlDraft` names); its metadata names
/// the control event and its payload identity the control payload. The host republishes that
/// control event through [`crate::plan_control_event_publication`], which requires the identical
/// draft and counts it once against the host capacity. For a host that publishes only on durably
/// committed branch values and keeps its other appends within
/// [`crate::ValidatedControlProjection::graph_room`], that republication fits for its own
/// publication whose compare-and-set did not commit, and of racing siblings whose joint demand
/// exceeds the capacity only a draft whose actual demand still fits republishes. A draft with
/// drafts a pipelining host chained below it may instead be refused with a capacity error (the
/// pipelined-chain residual of [`crate::ValidatedControlProjection::graph_room`]).
///
/// # Errors
///
/// Returns every error of [`crate::validate_control_advance`], and `Campaign(DuplicateControlEvent)`
/// when the pending draft names a control event the lineage already accepted (it can never be
/// republished on this head).
// r[impl campaign.publication_fence.pending_recovery]
// r[impl campaign.publication_fence.pending_several]
pub fn pending_control_draft(
    previous: &crate::contracts::ValidatedControlProjection,
    current: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<Option<PendingDraft>> {
    let validated = crate::fast_forward::validate_control_advance(previous, current)?;
    let Some(history_event_id) = validated.pending.first else {
        return Ok(None);
    };
    let content = current
        .graph
        .event(history_event_id)
        .ok_or(crate::contracts::AdapterError::MissingControlEvent)?
        .content();
    let control_event_id = crate::envelope::control_event_id(content, &validated.binding)
        .ok_or(crate::contracts::AdapterError::MetadataMismatch)?;
    if validated
        .state
        .event_ids
        .binary_search(&control_event_id)
        .is_ok()
    {
        return Err(campaign_core::CampaignError::DuplicateControlEvent.into());
    }
    debug_assert!(crate::envelope::is_control_child(
        content,
        &validated.binding,
        current.branch.head
    ));
    Ok(Some(PendingDraft {
        history_event: history_event_id,
        control_event: control_event_id,
        payload: crate::identity::id_from_digest(
            campaign_core::IdKind::EventPayload,
            content.payload_identity,
        ),
    }))
}
