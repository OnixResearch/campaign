//! A wrapper around the healthy Animus-shaped adapter with ninth-audit loopholes open.

/// One ninth-audit loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Loads and validates caches cold whenever the held projection has settled a selection or
    /// recorded a stop and the presented lineage is at least `gap` events longer than it (a host
    /// that resyncs a finished or settled campaign that fell far behind).
    GapResyncAfterSettlement { gap: usize },
    /// Loads and validates caches cold whenever the presented lineage is at least `gap` events
    /// longer than any held projection (the "resync on gap" shell).
    GapResync { gap: usize },
    /// Once the held projection has settled a selection or recorded a stop, answers a load
    /// Campaign refuses for any reason other than a non-fast-forward (and, when
    /// `spares_stale_graph`, other than a stale graph) with the held projection (a host that
    /// keeps serving its last good projection).
    LastGoodWhenSettled { spares_stale_graph: bool },
    /// Holding a stopped projection, answers every longer lineage Campaign loads with the held
    /// projection (a host that treats a stopped campaign as final).
    StoppedHolderIsFinal,
    /// Journals the recorded frontier and budget stops of every lineage and cache it loads (their
    /// event identities and stopped state identities) and claims exhaustive search on receipts
    /// naming them (a consumer that reports the search complete once Campaign records it).
    RecordedExhaustionClaim,
    /// Journals the event identities of recorded budget stops only and claims exhaustive search
    /// on their event receipts.
    RecordedBudgetEventClaim,
}

/// The healthy Animus-shaped adapter with the chosen loopholes open, and its journal of
/// recorded exhaustion stops.
pub(super) struct NinthAuditAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    exhausted: alloc::vec::Vec<campaign_core::Id>,
}

/// Whether a held projection has settled a selection or recorded a stop.
fn is_settled_or_stopped(held: &campaign_choregraph::ValidatedControlProjection) -> bool {
    !held.state().settled.is_empty() || held.state().stopped.is_some()
}

/// Whether a stop reason is a frontier or budget exhaustion.
const fn is_exhaustion(reason: &campaign_core::StopReason) -> bool {
    matches!(
        reason,
        campaign_core::StopReason::FrontierExhausted
            | campaign_core::StopReason::BudgetExhausted { .. }
    )
}

impl NinthAuditAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            exhausted: alloc::vec::Vec::new(),
        }
    }

    fn resyncs(
        &self,
        payloads: &[campaign_choregraph::ControlPayloadEnvelope],
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> bool {
        let Some(held) = previous else {
            return false;
        };
        let held_events = held.control_events().len();
        self.holes.iter().any(|hole| {
            let is_far = |gap: &usize| payloads.len() >= held_events.saturating_add(*gap);
            match hole {
                Hole::GapResyncAfterSettlement { gap } => {
                    is_settled_or_stopped(held) && is_far(gap)
                }
                Hole::GapResync { gap } => is_far(gap),
                _ => false,
            }
        })
    }

    /// Journals the recorded stops of `payloads` and of a loaded `state` the open holes key on.
    fn journal(
        &mut self,
        payloads: &[campaign_choregraph::ControlPayloadEnvelope],
        state: Option<&campaign_core::FrontierState>,
    ) {
        let is_exhaustion_claim = self.holes.contains(&Hole::RecordedExhaustionClaim);
        let is_budget_claim = self.holes.contains(&Hole::RecordedBudgetEventClaim);
        for payload in payloads {
            if let campaign_core::CampaignControlEvent::StopAccepted(stop) = &payload.control_event
            {
                let reason = &stop.stop.reason;
                let is_budget = matches!(reason, campaign_core::StopReason::BudgetExhausted { .. });
                let is_exhaustion_journalled = is_exhaustion_claim && is_exhaustion(reason);
                let is_budget_journalled = is_budget_claim && is_budget;
                let is_journalled = is_exhaustion_journalled || is_budget_journalled;
                if is_journalled && !self.exhausted.contains(&stop.event_id) {
                    self.exhausted.push(stop.event_id);
                }
            }
        }
        let stopped = state.filter(|state| {
            state
                .stopped
                .as_ref()
                .is_some_and(|stop| is_exhaustion(&stop.reason))
        });
        if let Some(state) = stopped.filter(|_| is_exhaustion_claim)
            && !self.exhausted.contains(&state.state_id)
        {
            self.exhausted.push(state.state_id);
        }
    }

    fn overclaims(&self, receipt: &campaign_core::CampaignReceipt) -> bool {
        self.exhausted.contains(&receipt.decision_id) || self.exhausted.contains(&receipt.state_id)
    }

    /// The held projection a last-good or stopped-is-final shell answers `loaded` with, if any.
    fn stale_answer(
        &self,
        held: &campaign_choregraph::ValidatedControlProjection,
        loaded: &campaign_choregraph::AdapterResult<
            campaign_choregraph::ValidatedControlProjection,
        >,
    ) -> Option<campaign_choregraph::ValidatedControlProjection> {
        let is_last_good = self.holes.iter().any(|hole| match hole {
            Hole::LastGoodWhenSettled { spares_stale_graph } => {
                loaded.as_ref().is_err_and(|error| {
                    *error != campaign_choregraph::AdapterError::NonFastForwardBranch
                        && !(*spares_stale_graph
                            && *error == campaign_choregraph::AdapterError::StaleGraph)
                })
            }
            _ => false,
        }) && is_settled_or_stopped(held);
        let is_final = self.holes.contains(&Hole::StoppedHolderIsFinal)
            && held.state().stopped.is_some()
            && loaded.as_ref().is_ok_and(|advanced| {
                advanced.control_events().len() > held.control_events().len()
            });
        (is_last_good || is_final).then(|| held.clone())
    }
}

impl crate::port::MomentMapping for NinthAuditAdapter {
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

impl crate::port::RecordMapping for NinthAuditAdapter {
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

impl crate::port::ConsumerShell for NinthAuditAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.resyncs(request.payloads, previous) {
            return campaign_choregraph::validate_control_projection(request);
        }
        let loaded = self.inner.load_history(request, previous);
        let state = loaded.as_ref().ok().map(|view| view.state().clone());
        self.journal(request.payloads, state.as_ref());
        if let Some(held) = previous
            && let Some(stale) = self.stale_answer(held, &loaded)
        {
            return Ok(stale);
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.resyncs(request.payloads, request.previous) {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            return self.inner.validate_cache(&cold, cache);
        }
        let validated = self.inner.validate_cache(request, cache);
        self.journal(request.payloads, validated.as_ref().ok());
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
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.overclaims(receipt) {
            claims.exhaustive_search = true;
        }
        claims
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
