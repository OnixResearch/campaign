//! The healthy Animus-shaped adapter with round-12 holes open. Every hole keys only on facts
//! Campaign shows the consumer: the held projection and the history it is handed, the receipts
//! it is asked about, and the states it loaded or validated.

/// What a load or cache hole sees: the held projection and the presented history.
pub(super) struct Presented<'p> {
    pub(super) held: &'p campaign_choregraph::ValidatedControlProjection,
    pub(super) payloads: &'p [campaign_choregraph::ControlPayloadEnvelope],
    pub(super) branch: &'p campaign_choregraph::BranchRef,
}

impl Presented<'_> {
    /// The held state.
    pub(super) fn state(&self) -> &campaign_core::FrontierState {
        self.held.state()
    }

    /// Whether the presented lineage is at least `events` events longer than the held one.
    pub(super) fn is_ahead(&self, events: usize) -> bool {
        self.payloads.len() >= self.held.control_events().len().saturating_add(events)
    }

    /// Whether the presented branch generation advanced past the held binding's.
    pub(super) fn is_advanced(&self) -> bool {
        self.branch.generation > self.held.binding().generation
    }

    /// The presented head event.
    pub(super) fn head(&self) -> Option<&campaign_core::CampaignControlEvent> {
        self.payloads.last().map(|payload| &payload.control_event)
    }

    /// The presented events after the held lineage's length.
    pub(super) fn new_events(&self) -> impl Iterator<Item = &campaign_core::CampaignControlEvent> {
        self.payloads
            .iter()
            .skip(self.held.control_events().len())
            .map(|payload| &payload.control_event)
    }
}

/// One hole.
#[derive(Clone, Copy, Debug)]
pub(super) enum Hole {
    /// Loads histories and validates caches cold (ignoring the held projection) whenever the key
    /// holds for the held projection and the presented history.
    ColdWhen(fn(&Presented<'_>) -> bool),
    /// Claims exhaustive search on the plan receipt of a planner stop planned at an earlier,
    /// running state once the state last loaded is stopped by a consumer or policy stop (a host
    /// that keeps the planner's stop receipt, records its own stop instead, and later reports
    /// the search exhaustive).
    StalePlannerStopClaim,
}

/// The healthy adapter with the chosen holes open, and the states its shell saw.
pub(super) struct ForkAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    states: alloc::vec::Vec<campaign_core::FrontierState>,
    latest: Option<campaign_core::FrontierState>,
}

impl ForkAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            states: alloc::vec::Vec::new(),
            latest: None,
        }
    }

    fn journal(&mut self, state: &campaign_core::FrontierState) {
        if !self
            .states
            .iter()
            .any(|known| known.state_id == state.state_id)
        {
            self.states.push(state.clone());
        }
    }

    fn is_cold(&self, presented: &Presented<'_>) -> bool {
        self.holes.iter().any(|hole| match hole {
            Hole::ColdWhen(key) => key(presented),
            Hole::StalePlannerStopClaim => false,
        })
    }

    /// Whether `receipt` is a planner stop's plan receipt of an unstopped state other than the
    /// state last loaded, which a consumer or policy stop stopped.
    fn is_stale_planner_stop(&self, receipt: &campaign_core::CampaignReceipt) -> bool {
        let Some(latest) = &self.latest else {
            return false;
        };
        let is_unrecorded = self
            .states
            .iter()
            .find(|state| state.state_id == receipt.state_id)
            .is_none_or(|planned| planned.stopped.is_none());
        let is_requested_stop = latest.stopped.as_ref().is_some_and(|stop| {
            matches!(
                stop.reason,
                campaign_core::StopReason::ConsumerRequested { .. }
                    | campaign_core::StopReason::PolicyRequested { .. }
            )
        });
        receipt.decision_id.kind == campaign_core::IdKind::Stop
            && is_unrecorded
            && latest.state_id != receipt.state_id
            && is_requested_stop
    }
}

impl crate::port::MomentMapping for ForkAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        self.inner.identities()
    }

    fn exploration_draft(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        parent_event: Option<campaign_choregraph::EventId>,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::EventDraft> {
        self.inner.exploration_draft(moment, parent_event)
    }

    fn moment_identity(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        event: campaign_choregraph::EventId,
    ) -> crate::verdicts::ConformanceResult<crate::port::MomentIdentity> {
        self.inner.moment_identity(moment, event)
    }

    fn reason(
        &mut self,
        reason: crate::vocabulary::ScenarioReason,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        self.inner.reason(reason)
    }
}

mod records;
