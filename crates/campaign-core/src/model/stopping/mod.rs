const LABEL_STOP: &[u8] = b"stop";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.stop]
pub enum StopReason {
    /// Public Campaign core API variant.
    FrontierExhausted,
    /// Public Campaign core API variant.
    BudgetExhausted {
        /// Budget dimensions that block all eligible candidates.
        dimensions: alloc::vec::Vec<crate::identity::Id>,
    },
    /// Public Campaign core API variant.
    ConsumerRequested {
        /// Consumer supplied bounded reason identity.
        reason_id: crate::identity::Id,
    },
    /// Public Campaign core API variant.
    PolicyRequested {
        /// Policy supplied bounded reason identity.
        reason_id: crate::identity::Id,
    },
    /// The campaign published `max_selections` selections and none remain outstanding.
    SelectionLimitReached,
    /// No selection is outstanding and either no control-event headroom remains for another
    /// selection, its settlement, and a stop, or no eligible candidate exists and an unguided
    /// candidate that fits its budget cannot receive guidance, a selection, that selection's
    /// settlement, and the stop within the control-event bound.
    ControlEventLimitReached,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct StopDecision {
    /// Public Campaign core API value.
    pub stop_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub reason: StopReason,
    /// Public Campaign core API value.
    pub incomplete_search_acknowledged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct StopEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub stop: StopDecision,
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
pub fn derive_stop_id(stop: &StopDecision) -> crate::error::CampaignResult<crate::identity::Id> {
    crate::identity::derive_id_with(crate::identity::IdKind::Stop, LABEL_STOP, |mut writer| {
        writer = writer.id(stop.campaign_id);
        writer = writer.id(stop.state_before_id);
        writer = match &stop.reason {
            StopReason::FrontierExhausted => writer.u32(STOP_FRONTIER),
            StopReason::BudgetExhausted { dimensions } => {
                let with_reason = writer.u32(STOP_BUDGET);
                let sorted = crate::identity::sorted_copy(dimensions);
                super::snapshots::write_ids(with_reason, &sorted)?
            }
            StopReason::ConsumerRequested { reason_id } => writer.u32(STOP_CONSUMER).id(*reason_id),
            StopReason::PolicyRequested { reason_id } => writer.u32(STOP_POLICY).id(*reason_id),
            StopReason::SelectionLimitReached => writer.u32(STOP_SELECTION_LIMIT),
            StopReason::ControlEventLimitReached => writer.u32(STOP_CONTROL_EVENT_LIMIT),
        };
        writer = writer.bool(stop.incomplete_search_acknowledged);
        Ok(writer)
    })
}

const STOP_FRONTIER: u32 = 1;

const STOP_BUDGET: u32 = 2;

const STOP_CONSUMER: u32 = 3;

const STOP_POLICY: u32 = 4;

const STOP_SELECTION_LIMIT: u32 = 5;

const STOP_CONTROL_EVENT_LIMIT: u32 = 6;
