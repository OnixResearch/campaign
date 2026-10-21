//! A wrapper around the healthy Animus-shaped adapter that misbehaves only on one input class the
//! suite once never presented (third closeout audit), or ignores the held projection of a cache
//! validation request.

mod lineage;

/// One input-class loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Own gate: durable flags, exact published decision, and `branch_before`; never checks the
    /// evidence's `history_event_id` or `branch_after`.
    SloppyEvidenceGate,
    /// Serves the held projection when a history is refused with a denial class the load matrix
    /// never presents (recorded-fence, envelope, parent, and Campaign errors).
    JournalOnUnpresentedLoadDenials,
    /// Reports only the first finding of a result.
    FirstFindingOnly,
    /// Claims exhaustive search on a stop receipt planned while the loaded campaign is not stopped
    /// (a planner stop such as frontier exhausted), never on the recorded stop.
    UnrecordedStopClaim,
    /// Advances by cold-start validation plus a length check (never shorter; equal length only at
    /// the held head) instead of the prefix/ancestry check.
    LengthMonotoneAdvance,
    /// Serves an internal journal on cold presentations refused as rewritten, altered, or
    /// ambiguous (presented by the suite only through the held projection).
    ColdJournalOnForgedDenials,
    /// Validates a cache by its binding, falling back to core replay of the presented payloads
    /// whenever lineage validation fails with anything but the presented lagging-payload denial.
    CacheFallbackOnLineageDenials,
    /// Validates every cache as a cold start, ignoring the request's held projection.
    ColdCacheIgnoringHeld,
}

pub(super) struct HoledAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    last_loaded_stopped: bool,
    journal: Option<campaign_choregraph::ValidatedControlProjection>,
}

impl HoledAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            last_loaded_stopped: false,
            journal: None,
        }
    }

    fn opens(&self, hole: Hole) -> bool {
        self.holes.contains(&hole)
    }

    /// The projection a holed shell serves in place of a refused load, if any.
    fn served_on_denial(
        &self,
        error: &campaign_choregraph::AdapterError,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> Option<campaign_choregraph::ValidatedControlProjection> {
        let is_forged = matches!(
            error,
            campaign_choregraph::AdapterError::PayloadIdentityMismatch
                | campaign_choregraph::AdapterError::ProjectionBindingMismatch
                | campaign_choregraph::AdapterError::History(_)
        );
        if self.opens(Hole::ColdJournalOnForgedDenials) && previous.is_none() && is_forged {
            return self.journal.clone();
        }
        if self.opens(Hole::JournalOnUnpresentedLoadDenials)
            && lineage::is_unpresented_load_denial(error)
        {
            return previous.cloned();
        }
        None
    }
}

impl crate::port::MomentMapping for HoledAdapter {
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

impl crate::port::RecordMapping for HoledAdapter {
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
        let mut observation = self.inner.result(outcome, context)?;
        if self.opens(Hole::FirstFindingOnly) {
            observation.findings.truncate(1);
        }
        Ok(observation)
    }
    fn closure(
        &mut self,
        context: &crate::port::ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        self.inner.closure(context)
    }
}

impl crate::port::ConsumerShell for HoledAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let loaded = if self.opens(Hole::LengthMonotoneAdvance) {
            lineage::length_monotone(request, previous)
        } else {
            self.inner.load_history(request, previous)
        };
        if let Ok(projection) = &loaded {
            self.last_loaded_stopped = projection.state().stopped.is_some();
            self.journal = Some(projection.clone());
        }
        if let Err(error) = &loaded
            && let Some(served) = self.served_on_denial(error, previous)
        {
            return Ok(served);
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.opens(Hole::ColdCacheIgnoringHeld) {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            return self.inner.validate_cache(&cold, cache);
        }
        let validated = self.inner.validate_cache(request, cache);
        if !self.opens(Hole::CacheFallbackOnLineageDenials) {
            return validated;
        }
        match validated {
            Err(
                campaign_choregraph::AdapterError::Campaign(
                    campaign_core::CampaignError::InvalidCache,
                )
                | campaign_choregraph::AdapterError::ProjectionPayloadMismatch,
            )
            | Ok(_) => validated,
            Err(_) => {
                let fresh = campaign_choregraph::derive_projection_binding(
                    request.graph,
                    request.branch,
                    request.projection_schema_id,
                    request.adapter_id,
                    request.control_domain_id,
                )?;
                let events: alloc::vec::Vec<campaign_core::CampaignControlEvent> = request
                    .payloads
                    .iter()
                    .map(|payload| payload.control_event.clone())
                    .collect();
                let state = campaign_core::reconstruct(&events)?;
                campaign_core::validate_cache_for_state(cache, &fresh, &state)?;
                Ok(state)
            }
        }
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        if !self.opens(Hole::SloppyEvidenceGate) {
            return self.inner.execution_gate(request);
        }
        let (Some(publication), Some(evidence)) = (request.publication, request.evidence) else {
            return crate::port::ExecutionAuthority::Refused;
        };
        let campaign_core::CampaignControlEvent::SelectionPublished(event) =
            publication.control_event()
        else {
            return crate::port::ExecutionAuthority::Refused;
        };
        let is_granted = evidence.durable_event
            && evidence.durable_branch_update
            && event.decision == *request.selection
            && evidence.branch_before == publication.branch_plan().before;
        if is_granted {
            crate::port::ExecutionAuthority::Granted
        } else {
            crate::port::ExecutionAuthority::Refused
        }
    }
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.opens(Hole::UnrecordedStopClaim)
            && receipt.decision_id.kind == campaign_core::IdKind::Stop
            && !self.last_loaded_stopped
        {
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
