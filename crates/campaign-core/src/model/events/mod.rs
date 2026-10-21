mod content;

const LABEL_EVENT: &[u8] = b"control-event";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub enum ControlEventKind {
    /// Public Campaign core API variant.
    CampaignAdmitted,
    /// Public Campaign core API variant.
    CandidatesAdmitted,
    /// Public Campaign core API variant.
    GuidanceAdmitted,
    /// Public Campaign core API variant.
    SelectionPublished,
    /// Public Campaign core API variant.
    ResultObserved,
    /// Public Campaign core API variant.
    UnobservedClosed,
    /// Public Campaign core API variant.
    CandidateRetired,
    /// Public Campaign core API variant.
    CandidatesPruned,
    /// Public Campaign core API variant.
    StopAccepted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub enum CampaignControlEvent {
    /// Public Campaign core API variant.
    CampaignAdmitted {
        /// Control event identity.
        event_id: crate::identity::Id,
        /// Campaign admission payload.
        admission: super::admission::CampaignAdmission,
    },
    /// Public Campaign core API variant.
    CandidatesAdmitted(super::candidates::CandidateBatchEvent),
    /// Public Campaign core API variant.
    GuidanceAdmitted(super::guidance::GuidanceEvent),
    /// Public Campaign core API variant.
    SelectionPublished(super::publication::SelectionEvent),
    /// Public Campaign core API variant.
    ResultObserved(super::observation::ResultEvent),
    /// Public Campaign core API variant.
    UnobservedClosed(super::termination::ClosureEvent),
    /// Public Campaign core API variant.
    CandidateRetired(super::termination::RetirementEvent),
    /// Public Campaign core API variant.
    CandidatesPruned(super::pruning::PruneEvent),
    /// Public Campaign core API variant.
    StopAccepted(super::stopping::StopEvent),
}

impl CampaignControlEvent {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[must_use]
    pub fn event_id(&self) -> crate::identity::Id {
        match self {
            Self::CampaignAdmitted { event_id, .. } => *event_id,
            Self::CandidatesAdmitted(event) => event.event_id,
            Self::GuidanceAdmitted(event) => event.event_id,
            Self::SelectionPublished(event) => event.event_id,
            Self::ResultObserved(event) => event.event_id,
            Self::UnobservedClosed(event) => event.event_id,
            Self::CandidateRetired(event) => event.event_id,
            Self::CandidatesPruned(event) => event.event_id,
            Self::StopAccepted(event) => event.event_id,
        }
    }

    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[must_use]
    pub fn state_before_id(&self) -> Option<crate::identity::Id> {
        match self {
            Self::CampaignAdmitted { .. } => None,
            Self::CandidatesAdmitted(event) => Some(event.state_before_id),
            Self::GuidanceAdmitted(event) => Some(event.state_before_id),
            Self::SelectionPublished(event) => Some(event.decision.state_before_id),
            Self::ResultObserved(event) => Some(event.state_before_id),
            Self::UnobservedClosed(event) => Some(event.state_before_id),
            Self::CandidateRetired(event) => Some(event.state_before_id),
            Self::CandidatesPruned(event) => Some(event.state_before_id),
            Self::StopAccepted(event) => Some(event.state_before_id),
        }
    }
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
pub fn derive_event_id(
    kind: &ControlEventKind,
    campaign_id: crate::identity::Id,
    state_before_id: crate::identity::Id,
    ids: &[crate::identity::Id],
) -> crate::error::CampaignResult<crate::identity::Id> {
    campaign_id.require_kind(crate::identity::IdKind::Campaign)?;
    crate::identity::derive_id_with(crate::identity::IdKind::Event, LABEL_EVENT, |mut writer| {
        writer = writer.u32(event_kind_code(kind));
        writer = writer.id(campaign_id);
        writer = writer.id(state_before_id);
        writer = writer.seq_len(crate::identity::usize_to_u64(ids.len())?);
        for id in ids {
            writer = writer.id(*id);
        }
        Ok(writer)
    })
}

const EVENT_CAMPAIGN: u32 = 1;

const EVENT_CANDIDATE: u32 = 2;

const EVENT_GUIDANCE: u32 = 3;

const EVENT_SELECTION: u32 = 4;

const EVENT_RESULT: u32 = 5;

const EVENT_CLOSURE: u32 = 6;

const EVENT_RETIREMENT: u32 = 7;

const EVENT_PRUNE: u32 = 8;

const EVENT_STOP: u32 = 9;

const fn event_kind_code(kind: &ControlEventKind) -> u32 {
    match kind {
        ControlEventKind::CampaignAdmitted => EVENT_CAMPAIGN,
        ControlEventKind::CandidatesAdmitted => EVENT_CANDIDATE,
        ControlEventKind::GuidanceAdmitted => EVENT_GUIDANCE,
        ControlEventKind::SelectionPublished => EVENT_SELECTION,
        ControlEventKind::ResultObserved => EVENT_RESULT,
        ControlEventKind::UnobservedClosed => EVENT_CLOSURE,
        ControlEventKind::CandidateRetired => EVENT_RETIREMENT,
        ControlEventKind::CandidatesPruned => EVENT_PRUNE,
        ControlEventKind::StopAccepted => EVENT_STOP,
    }
}
