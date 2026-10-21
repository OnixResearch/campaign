//! What the tenth-audit shell remembers of every lineage and cache it loads.

/// Identities the shell journals from the control events and states it loads.
#[derive(Default)]
pub(super) struct Journal {
    /// Accepted result events.
    pub(super) results: alloc::vec::Vec<campaign_core::Id>,
    /// Accepted result events that reported findings.
    pub(super) finding_results: alloc::vec::Vec<campaign_core::Id>,
    /// Accepted result events that reported child moments.
    pub(super) child_results: alloc::vec::Vec<campaign_core::Id>,
    /// Prune and retirement events.
    pub(super) deletions: alloc::vec::Vec<campaign_core::Id>,
    /// Stop events.
    pub(super) stop_events: alloc::vec::Vec<campaign_core::Id>,
    /// Host-capacity stop events.
    pub(super) capacity_events: alloc::vec::Vec<campaign_core::Id>,
    /// Stopped states with the class of their stop.
    stopped: alloc::vec::Vec<(campaign_core::Id, super::StopClass)>,
    /// Running states.
    pub(super) running: alloc::vec::Vec<campaign_core::Id>,
    /// Running states whose every candidate was selected and retired.
    pub(super) searched: alloc::vec::Vec<campaign_core::Id>,
    /// The state last loaded, when it is running.
    pub(super) held_running: Option<campaign_core::Id>,
}

/// Appends `id` once.
fn remember(ids: &mut alloc::vec::Vec<campaign_core::Id>, id: campaign_core::Id) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

/// The class of a stop reason.
fn class_of(reason: &campaign_core::StopReason) -> super::StopClass {
    match reason {
        campaign_core::StopReason::FrontierExhausted => super::StopClass::Frontier,
        campaign_core::StopReason::BudgetExhausted { .. } => super::StopClass::Budget,
        campaign_core::StopReason::SelectionLimitReached => super::StopClass::SelectionLimit,
        campaign_core::StopReason::ControlEventLimitReached => super::StopClass::ControlEventLimit,
        campaign_core::StopReason::PolicyRequested { .. } => super::StopClass::Policy,
        campaign_core::StopReason::ConsumerRequested { reason_id } => {
            if *reason_id == campaign_choregraph::control_capacity_reason_id() {
                super::StopClass::HostCapacity
            } else {
                super::StopClass::Consumer
            }
        }
    }
}

impl Journal {
    /// Whether `state` was loaded stopped by a stop of `class`.
    pub(super) fn is_stopped(&self, state: campaign_core::Id, class: super::StopClass) -> bool {
        self.stopped.contains(&(state, class))
    }

    /// Journals the control events of one loaded lineage and its loaded state.
    pub(super) fn record(
        &mut self,
        payloads: &[campaign_choregraph::ControlPayloadEnvelope],
        state: &campaign_core::FrontierState,
    ) {
        for payload in payloads {
            self.record_event(&payload.control_event);
        }
        if let Some(stop) = &state.stopped {
            let entry = (state.state_id, class_of(&stop.reason));
            if !self.stopped.contains(&entry) {
                self.stopped.push(entry);
            }
            self.held_running = None;
            return;
        }
        remember(&mut self.running, state.state_id);
        self.held_running = Some(state.state_id);
        let is_searched = state.selection_count > 0
            && !state.candidates.is_empty()
            && state.candidates.iter().all(|candidate| {
                matches!(
                    candidate.status,
                    campaign_core::CandidateStatus::Retired { .. }
                )
            });
        if is_searched {
            remember(&mut self.searched, state.state_id);
        }
    }

    fn record_event(&mut self, event: &campaign_core::CampaignControlEvent) {
        match event {
            campaign_core::CampaignControlEvent::ResultObserved(result) => {
                remember(&mut self.results, result.event_id);
                if !result.observation.findings.is_empty() {
                    remember(&mut self.finding_results, result.event_id);
                }
                if !result.observation.children.is_empty() {
                    remember(&mut self.child_results, result.event_id);
                }
            }
            campaign_core::CampaignControlEvent::CandidateRetired(retired) => {
                remember(&mut self.deletions, retired.event_id);
            }
            campaign_core::CampaignControlEvent::CandidatesPruned(pruned) => {
                remember(&mut self.deletions, pruned.event_id);
            }
            campaign_core::CampaignControlEvent::StopAccepted(stop) => {
                remember(&mut self.stop_events, stop.event_id);
                if class_of(&stop.stop.reason) == super::StopClass::HostCapacity {
                    remember(&mut self.capacity_events, stop.event_id);
                }
            }
            campaign_core::CampaignControlEvent::CampaignAdmitted { .. }
            | campaign_core::CampaignControlEvent::CandidatesAdmitted(_)
            | campaign_core::CampaignControlEvent::GuidanceAdmitted(_)
            | campaign_core::CampaignControlEvent::SelectionPublished(_)
            | campaign_core::CampaignControlEvent::UnobservedClosed(_) => {}
        }
    }
}
