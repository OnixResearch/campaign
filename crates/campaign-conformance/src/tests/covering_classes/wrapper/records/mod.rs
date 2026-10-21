//! Record mapping (delegated unchanged) and the holed consumer shell.

impl crate::port::RecordMapping for super::StateAdapter {
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

impl crate::port::ConsumerShell for super::StateAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let is_cold = previous.is_some_and(|held| self.is_cold(held));
        let loaded = if is_cold {
            campaign_choregraph::validate_control_projection(request)
        } else {
            self.inner.load_history(request, previous)
        };
        if let Ok(view) = &loaded {
            self.journal(request.payloads, view.state());
        }
        loaded
    }

    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        let is_cold = request.previous.is_some_and(|held| self.is_cold(held));
        let validated = if is_cold {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            self.inner.validate_cache(&cold, cache)
        } else {
            self.inner.validate_cache(request, cache)
        };
        if let Ok(state) = &validated {
            self.journal(request.payloads, state);
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
        let mut claims = self.inner.promotion_claims(receipt, signals);
        for hole in &self.holes {
            match hole {
                super::Hole::ColdWhen(_) => {}
                super::Hole::ClaimCleanExhaustion => {
                    let is_clean = self.is_clean_stop(receipt.state_id, |reason| {
                        matches!(
                            reason,
                            campaign_core::StopReason::FrontierExhausted
                                | campaign_core::StopReason::BudgetExhausted { .. }
                        )
                    });
                    claims.exhaustive_search |= is_clean;
                    claims.release_readiness |= is_clean;
                }
                super::Hole::ClaimCleanStopRelease => {
                    claims.release_readiness |= self.is_clean_stop(receipt.state_id, |_| true);
                }
                super::Hole::ClaimAfterStop(is_event, is_state) => {
                    let is_event_kind = self
                        .events
                        .iter()
                        .any(|event| event.event_id() == receipt.decision_id && is_event(event));
                    claims.safe_deletion |=
                        is_event_kind && self.state(receipt.state_id).is_some_and(is_state);
                }
            }
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
