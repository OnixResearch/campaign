//! A wrapper around the healthy Animus-shaped adapter with tenth-audit claim loopholes open.

mod observed;

/// The stop a stopped state records, by the class a consumer could key a claim on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StopClass {
    Frontier,
    Budget,
    SelectionLimit,
    ControlEventLimit,
    Policy,
    Consumer,
    HostCapacity,
}

/// One tenth-audit claim loophole. Each keys only on facts Campaign shows the consumer (the
/// control events and states it loads) and attaches one forbidden claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Claims exhaustive search on the event receipt of an accepted result projected against a
    /// state stopped by the frontier or budget stop (a consumer that re-issues its result
    /// receipts as "search complete" once the search stopped).
    ExhaustedResult,
    /// Claims exhaustive search on the event receipt of any control event but the stop itself
    /// projected against a state stopped by a stop of the class.
    StoppedEvent(StopClass),
    /// Claims exhaustive search on every receipt naming the adapter's host-capacity stop event or
    /// a state it stopped.
    CapacityStop,
    /// Claims finding correctness on the event receipt of a result that reported findings.
    FindingResult,
    /// Claims event truth on the event receipt of a result that reported child moments.
    ChildResult,
    /// Claims safe deletion on the event receipt of a prune or retirement projected against a
    /// running state.
    RunningDeletion,
    /// Claims operation success on the event receipt of a result projected against the running
    /// state the shell last loaded (a consumer that trusts a freshly published result).
    HeldRunningResult,
    /// Claims exhaustive search on a stop plan receipt of a running state whose every candidate
    /// was selected and retired (the frontier stop of a searched frontier before it is recorded).
    UnrecordedSearchedFrontier,
}

/// The healthy Animus-shaped adapter with the chosen loopholes open, and its journal.
pub(super) struct TenthAuditAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    journal: observed::Journal,
}

impl TenthAuditAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            journal: observed::Journal::default(),
        }
    }

    /// The claims the open holes attach to `receipt`.
    fn overclaim(
        &self,
        receipt: &campaign_core::CampaignReceipt,
        mut claims: campaign_core::ReceiptClaims,
    ) -> campaign_core::ReceiptClaims {
        let journal = &self.journal;
        let decision = receipt.decision_id;
        let state = receipt.state_id;
        let is_event = decision.kind == campaign_core::IdKind::Event;
        for hole in &self.holes {
            match hole {
                Hole::ExhaustedResult => {
                    let is_exhausted = journal.is_stopped(state, StopClass::Frontier)
                        || journal.is_stopped(state, StopClass::Budget);
                    claims.exhaustive_search |= is_exhausted && journal.results.contains(&decision);
                }
                Hole::StoppedEvent(class) => {
                    claims.exhaustive_search |= is_event
                        && journal.is_stopped(state, *class)
                        && !journal.stop_events.contains(&decision);
                }
                Hole::CapacityStop => {
                    claims.exhaustive_search |= journal.capacity_events.contains(&decision)
                        || journal.is_stopped(state, StopClass::HostCapacity);
                }
                Hole::FindingResult => {
                    claims.finding_correctness |= journal.finding_results.contains(&decision);
                }
                Hole::ChildResult => {
                    claims.event_truth |= journal.child_results.contains(&decision);
                }
                Hole::RunningDeletion => {
                    claims.safe_deletion |=
                        journal.deletions.contains(&decision) && journal.running.contains(&state);
                }
                Hole::HeldRunningResult => {
                    claims.operation_success |=
                        journal.held_running == Some(state) && journal.results.contains(&decision);
                }
                Hole::UnrecordedSearchedFrontier => {
                    claims.exhaustive_search |= decision.kind == campaign_core::IdKind::Stop
                        && journal.searched.contains(&state);
                }
            }
        }
        claims
    }
}

impl crate::port::MomentMapping for TenthAuditAdapter {
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

impl crate::port::RecordMapping for TenthAuditAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        self.inner.candidate(operation, context)
    }
    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        self.inner.guidance(preference, context)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        self.inner.revision(preference, context)
    }
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        self.inner.entropy_ticket(draw, context)
    }
    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        self.inner.result(outcome, context)
    }
    fn closure(
        &mut self,
        context: &crate::port::ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        self.inner.closure(context)
    }
}

impl crate::port::ConsumerShell for TenthAuditAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let loaded = self.inner.load_history(request, previous);
        if let Ok(view) = &loaded {
            self.journal.record(request.payloads, view.state());
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        let validated = self.inner.validate_cache(request, cache);
        if let Ok(state) = &validated {
            self.journal.record(request.payloads, state);
        }
        validated
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        self.inner.execution_gate(request)
    }
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let claims = self.inner.promotion_claims(receipt, signals);
        self.overclaim(receipt, claims)
    }
    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture {
        self.inner.role_identities(candidate, work_item, result)
    }
}
