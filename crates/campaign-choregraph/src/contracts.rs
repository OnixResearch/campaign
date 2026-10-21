//! Public contracts for the Campaign-to-Choregraph adapter.

mod planned;
mod view;

pub use crate::contracts::planned::ControlPublication;
pub use crate::contracts::view::ValidatedControlProjection;

/// Adapter result type.
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Fail-closed adapter errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError {
    /// The Campaign core rejected a value.
    Campaign(campaign_core::CampaignError),
    /// Choregraph rejected a graph, projection, or branch plan.
    History(choregraph_history::HistoryError),
    /// A Choregraph event is not an application envelope.
    NonApplicationEnvelope,
    /// The event payload identity does not match the Campaign control payload.
    PayloadIdentityMismatch,
    /// The Choregraph event schema is not the expected Campaign schema.
    SchemaIdentityMismatch,
    /// The Choregraph projection identity is not the expected control domain.
    ProjectionIdentityMismatch,
    /// The Choregraph metadata does not bind the exact Campaign identities.
    MetadataMismatch,
    /// The projected event count and supplied payload count differ.
    ProjectionPayloadMismatch,
    /// The caller supplied a projection identity that does not match the graph.
    ProjectionBindingMismatch,
    /// The lineage did not contain the requested control event.
    MissingControlEvent,
    /// The event parent set is not the requested linear control lineage.
    ParentLineageMismatch,
    /// The durable publication evidence does not match the planned publication.
    PublicationEvidenceMismatch,
    /// A published selection's planned fence does not equal the current branch value.
    PublicationFenceMismatch,
    /// The control branch is not the branch named by the Campaign admission.
    ControlBranchMismatch,
    /// The control branch generation is not the admission generation plus the control-lineage
    /// length (the branch was rewound or advanced without publishing a control event), or the
    /// lineage does not extend the previously validated projection (the branch moved onto a fork).
    NonFastForwardBranch,
    /// Publishing the event would leave the control lineage, its outstanding settlements, and the
    /// stop reserve beyond the host's Choregraph projection bound (`maximum_events`); a
    /// republication also counts each draft chained below it as a selection.
    ProjectionCapacityExhausted,
    /// Publishing the event would leave the control branch generation, its outstanding
    /// settlements, and the stop reserve beyond `u64::MAX`; a republication also counts each
    /// draft chained below it as a selection.
    GenerationCapacityExhausted,
    /// Publishing the event would leave the Choregraph graph, its outstanding settlements, and the
    /// stop reserve beyond the graph's own event limit (`HistoryLimits::max_events`); a
    /// republication also counts each draft chained below it as a selection.
    GraphCapacityExhausted,
    /// The graph already holds another Campaign control draft whose parent is the presented
    /// branch head (a publication whose branch update is pending): it must be completed (D56
    /// republication) before another event is planned or published on that head. With several
    /// such drafts, the first in Choregraph event-identity order is named. Publication chains
    /// drafts below it only for a host that pipelined (published on a branch value whose
    /// compare-and-set was not durable); they are republished after it, in chain order, where they
    /// still fit, and a chain whose republication no longer fits keeps the head blocked with this
    /// error (the pipelined-chain residual of [`crate::ValidatedControlProjection::graph_room`]).
    PendingControlDraft {
        /// The pending draft's Choregraph event identity.
        event_id: choregraph_history::EventId,
    },
    /// The graph holds another Campaign campaign under the projection's envelope (projection
    /// schema, adapter, and control domain): one Campaign campaign per graph and envelope. Root
    /// admission refuses a root while the graph holds any other Campaign control event under the
    /// envelope; validation refuses a graph holding a control root other than the lineage's.
    SharedControlGraph {
        /// The first such event in Choregraph event-identity order.
        event_id: choregraph_history::EventId,
    },
    /// The presented graph lacks events the held projection recorded (it holds fewer events than
    /// the held projection's graph, or lacks a Campaign control draft the held projection
    /// recorded as pending): a fast-forward never goes back in graph time.
    StaleGraph,
}

impl From<campaign_core::CampaignError> for AdapterError {
    fn from(error: campaign_core::CampaignError) -> Self {
        Self::Campaign(error)
    }
}

impl From<choregraph_history::HistoryError> for AdapterError {
    fn from(error: choregraph_history::HistoryError) -> Self {
        Self::History(error)
    }
}

/// One Campaign control payload bound to one immutable Choregraph event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPayloadEnvelope {
    /// The Choregraph application envelope identity.
    pub history_event_id: choregraph_history::EventId,
    /// The exact Campaign control event payload.
    pub control_event: campaign_core::CampaignControlEvent,
}

/// Request to validate one caller-selected linear control projection.
pub struct ControlProjectionRequest<'a> {
    /// The current immutable Choregraph graph.
    pub graph: &'a choregraph_history::HistoryGraph,
    /// The generation-fenced control branch.
    pub branch: &'a choregraph_history::BranchRef,
    /// Explicit parent choices for every multi-parent history event.
    pub parent_choices: &'a choregraph_history::ParentChoices,
    /// Maximum events admitted to the Choregraph linear projection.
    pub maximum_events: u32,
    /// Exact Campaign binding that the caller expects for this projection.
    pub expected_binding: campaign_core::ProjectionBinding,
    /// Control payloads in the same order as the selected linear projection.
    pub payloads: &'a [ControlPayloadEnvelope],
}

/// Request to publish one Campaign control event into Choregraph history.
///
/// Publication always advances the projection the event was computed from; there is no
/// cold-start publication path, so a host cannot publish on a history that its held projection
/// refuses:
///
/// ```compile_fail
/// fn cold_start<'a>(
///     current: campaign_choregraph::ControlProjectionRequest<'a>,
///     control_event: campaign_core::CampaignControlEvent,
/// ) -> campaign_choregraph::ControlPublicationRequest<'a> {
///     campaign_choregraph::ControlPublicationRequest {
///         current,
///         previous: None,
///         control_event,
///     }
/// }
/// ```
pub struct ControlPublicationRequest<'a> {
    /// The caller's current control projection inputs; the adapter re-validates them and applies
    /// the event to the reconstructed state before admitting the draft.
    pub current: ControlProjectionRequest<'a>,
    /// The validated projection the event was computed from. The current inputs are always
    /// validated as its fast-forward through [`crate::validate_control_advance`]; a host without
    /// a held projection first loads one through [`crate::validate_control_projection`], the only
    /// cold-start entry.
    pub previous: &'a ValidatedControlProjection,
    /// The Campaign control event to wrap as an application envelope.
    pub control_event: campaign_core::CampaignControlEvent,
}

/// One immutable root admission of a Campaign control event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlRootAdmission {
    /// The Campaign payload envelope for the admitted event.
    pub envelope: ControlPayloadEnvelope,
    /// The immutable graph after root event admission.
    pub graph_after_admission: choregraph_history::HistoryGraph,
}

/// Request to plan against a validated published Choregraph projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedSelectionRequest {
    /// The Campaign selection request supplied by the caller.
    pub request: campaign_core::SelectionRequest,
}

/// Durable-publication facts supplied by a caller after its storage operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurablePublicationEvidence {
    /// The Choregraph event observed as durably admitted by the caller.
    pub history_event_id: choregraph_history::EventId,
    /// The branch value before the compare-and-set operation.
    pub branch_before: choregraph_history::BranchRef,
    /// The branch value after the compare-and-set operation.
    pub branch_after: choregraph_history::BranchRef,
    /// True only when the caller observed durable event admission.
    pub durable_event: bool,
    /// True only when the caller observed durable branch mutation.
    pub durable_branch_update: bool,
}
