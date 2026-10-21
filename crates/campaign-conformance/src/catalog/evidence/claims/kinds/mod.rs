//! Forbidden effects of claimed event receipts, one per control event variant and stop reason.

/// Forbidden effect of a claimed event receipt of one control event variant.
pub(super) const fn event_kind(event: &campaign_core::CampaignControlEvent) -> &'static str {
    match event {
        campaign_core::CampaignControlEvent::CampaignAdmitted { .. } => {
            "a campaign admission event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::CandidatesAdmitted(_) => {
            "an event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::GuidanceAdmitted(_) => {
            "a guidance event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::SelectionPublished(_) => {
            "a selection event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::ResultObserved(_) => {
            "a result event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::UnobservedClosed(_) => {
            "a closure event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::CandidateRetired(_) => {
            "a retirement event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::CandidatesPruned(_) => {
            "a prune event receipt carried a promotion claim"
        }
        campaign_core::CampaignControlEvent::StopAccepted(stop) => stop_kind(&stop.stop.reason),
    }
}

/// Forbidden effect of a claimed event receipt of a stop recorded for `reason` (a consumer stop,
/// the adapter's host-capacity stop included, under one name).
const fn stop_kind(reason: &campaign_core::StopReason) -> &'static str {
    match reason {
        campaign_core::StopReason::FrontierExhausted => {
            "a frontier-exhausted stop event receipt carried a promotion claim"
        }
        campaign_core::StopReason::BudgetExhausted { .. } => {
            "a budget-exhausted stop event receipt carried a promotion claim"
        }
        campaign_core::StopReason::SelectionLimitReached => {
            "a selection-limit stop event receipt carried a promotion claim"
        }
        campaign_core::StopReason::ControlEventLimitReached => {
            "a control-event-limit stop event receipt carried a promotion claim"
        }
        campaign_core::StopReason::PolicyRequested { .. } => {
            "a policy stop event receipt carried a promotion claim"
        }
        campaign_core::StopReason::ConsumerRequested { .. } => {
            "a stop event receipt carried a promotion claim"
        }
    }
}
