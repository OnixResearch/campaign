//! Adapter-independent labels of Campaign decisions and denials.

/// Label of a typed denial; history errors and the adapter's pending-draft and shared-graph
/// refusals carry adapter-specific event identities and are reported by class only.
pub(super) fn denial(error: &crate::verdicts::ConformanceError) -> alloc::string::String {
    if let Some(class) = identified_class(error) {
        return alloc::string::String::from(class);
    }
    if let Some(campaign) = error.campaign() {
        return alloc::format!("{campaign:?}");
    }
    alloc::format!("{error:?}")
}

/// Class of a denial that names adapter-specific event identities.
const fn identified_class(error: &crate::verdicts::ConformanceError) -> Option<&'static str> {
    match error {
        crate::verdicts::ConformanceError::History(_)
        | crate::verdicts::ConformanceError::Adapter(campaign_choregraph::AdapterError::History(
            _,
        )) => Some("history"),
        crate::verdicts::ConformanceError::Adapter(
            campaign_choregraph::AdapterError::PendingControlDraft { .. },
        ) => Some("pending-control-draft"),
        crate::verdicts::ConformanceError::Adapter(
            campaign_choregraph::AdapterError::SharedControlGraph { .. },
        ) => Some("shared-control-graph"),
        _ => None,
    }
}

/// Label of a planner wait.
pub(super) const fn wait(reason: &campaign_core::WaitReason) -> &'static str {
    match reason {
        campaign_core::WaitReason::AwaitingGuidance => "awaiting-guidance",
        campaign_core::WaitReason::AwaitingResults => "awaiting-results",
        campaign_core::WaitReason::AwaitingCapacity => "awaiting-capacity",
    }
}

/// Label of a guidance dynamic.
pub(super) const fn dynamics(dynamics: &campaign_core::GuidanceDynamics) -> &'static str {
    match dynamics {
        campaign_core::GuidanceDynamics::Discovery => "discovery",
        campaign_core::GuidanceDynamics::StrictImprovement { .. } => "strict-improvement",
        campaign_core::GuidanceDynamics::CoalescedDuplicate { .. } => "coalesced-duplicate",
        campaign_core::GuidanceDynamics::Revision { .. } => "revision",
    }
}

/// Label of a stop class; budget dimensions are named by their declared index.
pub(super) fn stop(
    reason: &campaign_core::StopReason,
    dimensions: &[campaign_core::Id],
) -> alloc::string::String {
    match reason {
        campaign_core::StopReason::FrontierExhausted => {
            alloc::string::String::from("frontier-exhausted")
        }
        campaign_core::StopReason::BudgetExhausted {
            dimensions: exhausted,
        } => {
            let mut indices: alloc::vec::Vec<usize> = exhausted
                .iter()
                .filter_map(|id| dimensions.iter().position(|known| known == id))
                .collect();
            indices.sort_unstable();
            alloc::format!("budget-exhausted{indices:?}")
        }
        campaign_core::StopReason::ConsumerRequested { .. } => {
            alloc::string::String::from("consumer-requested")
        }
        campaign_core::StopReason::PolicyRequested { .. } => {
            alloc::string::String::from("policy-requested")
        }
        campaign_core::StopReason::SelectionLimitReached => {
            alloc::string::String::from("selection-limit-reached")
        }
        campaign_core::StopReason::ControlEventLimitReached => {
            alloc::string::String::from("control-event-limit-reached")
        }
    }
}
