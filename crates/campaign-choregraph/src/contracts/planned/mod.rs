//! The planned publication of one Campaign control event.

/// Planned immutable publication of one Campaign control event.
///
/// Only [`crate::plan_control_event_publication`] constructs this value, so its draft, event
/// identity, and branch plan always encode its control event:
///
/// ```compile_fail
/// fn splice(
///     publication: campaign_choregraph::ControlPublication,
///     control_event: campaign_core::CampaignControlEvent,
/// ) -> campaign_choregraph::ControlPublication {
///     campaign_choregraph::ControlPublication {
///         control_event,
///         ..publication
///     }
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPublication {
    pub(crate) payload_id: campaign_core::Id,
    pub(crate) draft: choregraph_history::EventDraft,
    pub(crate) history_event_id: choregraph_history::EventId,
    pub(crate) graph_after_admission: choregraph_history::HistoryGraph,
    pub(crate) branch_plan: choregraph_history::ReferenceMutationPlan,
    pub(crate) control_event: campaign_core::CampaignControlEvent,
    pub(crate) binding: campaign_core::ProjectionBinding,
}

impl ControlPublication {
    /// The Campaign control payload identity.
    #[must_use]
    pub const fn payload_id(&self) -> campaign_core::Id {
        self.payload_id
    }

    /// The Choregraph application draft.
    #[must_use]
    pub const fn draft(&self) -> &choregraph_history::EventDraft {
        &self.draft
    }

    /// The Choregraph event identity.
    #[must_use]
    pub const fn history_event_id(&self) -> choregraph_history::EventId {
        self.history_event_id
    }

    /// The immutable graph after event admission.
    #[must_use]
    pub const fn graph_after_admission(&self) -> &choregraph_history::HistoryGraph {
        &self.graph_after_admission
    }

    /// The generation-fenced branch mutation plan.
    #[must_use]
    pub const fn branch_plan(&self) -> &choregraph_history::ReferenceMutationPlan {
        &self.branch_plan
    }

    /// The Campaign control event that the draft contains.
    #[must_use]
    pub const fn control_event(&self) -> &campaign_core::CampaignControlEvent {
        &self.control_event
    }

    /// The validated projection binding of the branch value the publication advances.
    #[must_use]
    pub const fn binding(&self) -> &campaign_core::ProjectionBinding {
        &self.binding
    }
}
