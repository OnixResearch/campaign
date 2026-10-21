//! A wrapper around the healthy Animus-shaped adapter with seventh-audit loopholes open.

/// One seventh-audit loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Keeps the first and the last level of every score of three or more levels and zeroes
    /// every level between them (a consumer that stores a primary key, a tie-break key, and
    /// drops the rest).
    ZeroMiddleLevels,
    /// Sorts the levels of every score in descending order (a consumer that keeps its score keys
    /// in an ordered set and reads them largest first).
    SortLevelsDescending,
    /// Closes the selection of the executed candidate it last saw granted (a consumer whose
    /// in-flight work queue is keyed by candidate), the closure mirror of the caught
    /// `ResultKeyedByCandidate` result control.
    ClosureKeyedByCandidate,
    /// Reduces both scenario draws modulo one million before building the ticket (a
    /// parts-per-million consumer).
    NarrowIndexDraw,
    /// Loads and validates caches cold whenever the held projection has no outstanding
    /// selection (a host that only keeps its held projection while work is in flight).
    ColdWhenIdle,
}

/// The healthy Animus-shaped adapter with the chosen loopholes open.
pub(super) struct UnsampledOrderAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    granted: alloc::vec::Vec<campaign_core::SelectionDecision>,
}

/// Units in one scenario draw a parts-per-million consumer keeps.
const PARTS_PER_MILLION: u64 = 1_000_000;

impl UnsampledOrderAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            granted: alloc::vec::Vec::new(),
        }
    }

    fn resyncs(&self, previous: Option<&campaign_choregraph::ValidatedControlProjection>) -> bool {
        self.holes.contains(&Hole::ColdWhenIdle)
            && previous.is_some_and(|held| held.state().outstanding.is_empty())
    }

    fn kept(
        &self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::vocabulary::ScenarioPreference {
        let mut kept = preference.clone();
        if self.holes.contains(&Hole::ZeroMiddleLevels) {
            let last = kept.score.levels.len().saturating_sub(1);
            for (index, level) in kept.score.levels.iter_mut().enumerate() {
                if index > 0 && index < last {
                    *level = 0;
                }
            }
        }
        if self.holes.contains(&Hole::SortLevelsDescending) {
            kept.score.levels.sort_unstable_by(|a, b| b.cmp(a));
        }
        kept
    }
}

impl crate::port::MomentMapping for UnsampledOrderAdapter {
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

impl crate::port::RecordMapping for UnsampledOrderAdapter {
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
        let kept = self.kept(preference);
        self.inner.guidance(&kept, context)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let kept = self.kept(preference);
        self.inner.revision(&kept, context)
    }
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        if self.holes.contains(&Hole::NarrowIndexDraw) {
            let narrowed = crate::vocabulary::ScenarioDraw {
                explore_draw: draw.explore_draw % PARTS_PER_MILLION,
                index_draw: draw.index_draw % PARTS_PER_MILLION,
            };
            return self.inner.entropy_ticket(&narrowed, context);
        }
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
        if self.holes.contains(&Hole::ClosureKeyedByCandidate)
            && let Some(keyed) = self
                .granted
                .iter()
                .rev()
                .find(|granted| granted.candidate_id == context.selection.candidate_id)
                .cloned()
        {
            let keyed_context = crate::port::ClosureContext {
                campaign_id: context.campaign_id,
                selection: &keyed,
            };
            return self.inner.closure(&keyed_context);
        }
        self.inner.closure(context)
    }
}

impl crate::port::ConsumerShell for UnsampledOrderAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.resyncs(previous) {
            return campaign_choregraph::validate_control_projection(request);
        }
        self.inner.load_history(request, previous)
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.resyncs(request.previous) {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            return self.inner.validate_cache(&cold, cache);
        }
        self.inner.validate_cache(request, cache)
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        let authority = self.inner.execution_gate(request);
        if authority == crate::port::ExecutionAuthority::Granted {
            self.granted.push(request.selection.clone());
        }
        authority
    }
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        self.inner.promotion_claims(receipt, signals)
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
