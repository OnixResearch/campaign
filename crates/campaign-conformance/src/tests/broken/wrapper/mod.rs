//! A wrapper around the healthy Animus-shaped adapter that breaks exactly one contract.

/// The one contract a wrapper breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Breakage {
    /// No breakage: every call is delegated.
    Healthy,
    /// Trusts a derived cache instead of validating it against current history.
    StaleHistory,
    /// Grants execution without Campaign's publication gate.
    ExecutionBeforePublication,
    /// Attributes an execution's result to another outstanding selection.
    CrossedIdentities,
    /// Declares one unit for every operation regardless of its cost.
    BudgetMiscount,
    /// Serves its own journal of the last loaded projection instead of the presented history
    /// whenever that history fails validation.
    HistoryRewrite,
    /// Turns consumer signals into receipt claims.
    ReceiptOverclaim,
    /// Reuses the candidate identity for its work item and result.
    RoleCollapse,
}

/// Healthy adapter wrapped with one breakage.
pub(super) struct FaultyAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    breakage: Breakage,
    journal: Option<campaign_choregraph::ValidatedControlProjection>,
}

impl FaultyAdapter {
    pub(super) const fn new(breakage: Breakage) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            breakage,
            journal: None,
        }
    }
}

impl crate::port::MomentMapping for FaultyAdapter {
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

impl crate::port::RecordMapping for FaultyAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        let mut record = self.inner.candidate(operation, context)?;
        if self.breakage == Breakage::BudgetMiscount {
            for cost in &mut record.declared_costs {
                cost.amount = 1;
            }
        }
        Ok(record)
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
        let other = context
            .state
            .outstanding
            .iter()
            .find(|outstanding| outstanding.selection_id != context.selection.selection_id);
        if let (Breakage::CrossedIdentities, Some(other)) = (self.breakage, other) {
            observation.selection_id = other.selection_id;
            observation.parent_moment_id = other.parent_moment_id;
            observation.operation_id = other.operation_id;
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

impl crate::port::ConsumerShell for FaultyAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.breakage != Breakage::HistoryRewrite {
            return self.inner.load_history(request, previous);
        }
        match self.inner.load_history(request, previous) {
            Ok(loaded) => {
                self.journal = Some(loaded.clone());
                Ok(loaded)
            }
            Err(error) => self.journal.clone().ok_or(error),
        }
    }

    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.breakage != Breakage::StaleHistory {
            return self.inner.validate_cache(request, cache);
        }
        let cached = usize::try_from(cache.control_event_count)
            .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
        let events: alloc::vec::Vec<campaign_core::CampaignControlEvent> = request
            .payloads
            .iter()
            .take(cached)
            .map(|payload| payload.control_event.clone())
            .collect();
        Ok(campaign_core::reconstruct(&events)?)
    }

    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        if self.breakage == Breakage::ExecutionBeforePublication {
            return crate::port::ExecutionAuthority::Granted;
        }
        self.inner.execution_gate(request)
    }

    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.breakage == Breakage::ReceiptOverclaim {
            claims.operation_success = signals.has_zero_exit_status;
            claims.release_readiness = signals.is_top_ranked && signals.is_selected;
            claims.authorization = signals.has_check_reference;
        }
        claims
    }

    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture {
        let mut roles = self.inner.role_identities(candidate, work_item, result);
        if self.breakage == Breakage::RoleCollapse {
            roles.queued_work_id = candidate.candidate_id;
            roles.computed_result_id = candidate.candidate_id;
        }
        roles
    }
}
