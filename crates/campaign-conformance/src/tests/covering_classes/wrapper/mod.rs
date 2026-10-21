//! The healthy Animus-shaped adapter with round-11 state-keyed holes open. Every hole keys only
//! on facts Campaign shows the consumer: the held projection it is handed, and the lineages and
//! states it loaded or validated.

/// A fact of a held state a shell keys a cold reload on.
#[derive(Clone, Copy, Debug)]
pub(super) enum Held {
    /// Stopped with nothing outstanding.
    Finished,
    /// Some candidate retired.
    Retired,
    /// Some candidate pruned.
    Pruned,
    /// Some candidate retired or pruned.
    RetiredOrPruned,
    /// Some child moment discovered.
    Discoveries,
    /// Every one of four facts.
    All([fn(&campaign_core::FrontierState) -> bool; 4]),
}

impl Held {
    /// Whether `state` has the fact.
    fn matches(self, state: &campaign_core::FrontierState) -> bool {
        let any = |is: fn(&campaign_core::CandidateStatus) -> bool| {
            state
                .candidates
                .iter()
                .any(|candidate| is(&candidate.status))
        };
        if let Self::All(facts) = self {
            facts.iter().all(|fact| fact(state))
        } else if matches!(self, Self::Finished) {
            state.stopped.is_some() && state.outstanding.is_empty()
        } else if matches!(self, Self::Retired) {
            any(|status| matches!(status, campaign_core::CandidateStatus::Retired { .. }))
        } else if matches!(self, Self::Pruned) {
            any(|status| matches!(status, campaign_core::CandidateStatus::Pruned { .. }))
        } else if matches!(self, Self::RetiredOrPruned) {
            any(|status| !matches!(status, campaign_core::CandidateStatus::Retained))
        } else {
            !state.discoveries.is_empty()
        }
    }
}

/// One hole.
#[derive(Clone, Copy, Debug)]
pub(super) enum Hole {
    /// Loads histories and validates caches cold (ignoring the held projection) whenever the held
    /// state has the fact.
    ColdWhen(Held),
    /// Claims exhaustive search and release readiness on every receipt projected against a state
    /// a frontier or budget stop stopped after the campaign selected work and accepted no
    /// finding.
    ClaimCleanExhaustion,
    /// Claims release readiness on every receipt projected against a state stopped for any
    /// reason after the campaign selected work and accepted no finding.
    ClaimCleanStopRelease,
    /// Claims safe deletion on the event receipt of an event with the first fact projected
    /// against a state with the second (a stopped state of one reason).
    ClaimAfterStop(
        fn(&campaign_core::CampaignControlEvent) -> bool,
        fn(&campaign_core::FrontierState) -> bool,
    ),
}

/// The healthy adapter with the chosen holes open, and what its shell journals.
pub(super) struct StateAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    states: alloc::vec::Vec<campaign_core::FrontierState>,
    events: alloc::vec::Vec<campaign_core::CampaignControlEvent>,
}

impl StateAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            states: alloc::vec::Vec::new(),
            events: alloc::vec::Vec::new(),
        }
    }

    fn journal(
        &mut self,
        payloads: &[campaign_choregraph::ControlPayloadEnvelope],
        state: &campaign_core::FrontierState,
    ) {
        for payload in payloads {
            if !self.events.contains(&payload.control_event) {
                self.events.push(payload.control_event.clone());
            }
        }
        if !self
            .states
            .iter()
            .any(|known| known.state_id == state.state_id)
        {
            self.states.push(state.clone());
        }
    }

    fn state(&self, state_id: campaign_core::Id) -> Option<&campaign_core::FrontierState> {
        self.states.iter().find(|state| state.state_id == state_id)
    }

    fn is_cold(&self, held: &campaign_choregraph::ValidatedControlProjection) -> bool {
        self.holes.iter().any(|hole| match hole {
            Hole::ColdWhen(fact) => fact.matches(held.state()),
            _ => false,
        })
    }

    /// Whether `state_id` names a journaled state stopped by a stop `is_reason` accepts after
    /// the campaign selected work and accepted no finding.
    fn is_clean_stop(
        &self,
        state_id: campaign_core::Id,
        is_reason: fn(&campaign_core::StopReason) -> bool,
    ) -> bool {
        self.state(state_id).is_some_and(|state| {
            state
                .stopped
                .as_ref()
                .is_some_and(|stop| is_reason(&stop.reason))
                && state.selection_count > 0
                && state.findings.is_empty()
        })
    }
}

impl crate::port::MomentMapping for StateAdapter {
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
