//! The consumer shell of the unpresented-class wrapper: the non-selection-publication gate, the
//! gap-resync load and cache paths, and the held projection served on refused loads.

/// Authority of one own gate decision.
const fn authority(is_granted: bool) -> crate::port::ExecutionAuthority {
    if is_granted {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}

/// The non-selection-publication gate: trusts durable evidence matching a non-selection
/// publication's own planned transition.
fn non_selection_gate(
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
) -> crate::port::ExecutionAuthority {
    let plan = publication.branch_plan();
    authority(
        evidence.durable_event
            && evidence.durable_branch_update
            && evidence.history_event_id == publication.history_event_id()
            && evidence.branch_before == plan.before
            && evidence.branch_after == plan.after,
    )
}

impl crate::port::ConsumerShell for super::UnpresentedClassAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.resyncs(request.payloads.len(), previous) {
            return campaign_choregraph::validate_control_projection(request);
        }
        let loaded = self.inner.load_history(request, previous);
        let is_exceeded = matches!(
            loaded,
            Err(campaign_choregraph::AdapterError::History(
                campaign_choregraph::HistoryError::ProjectionLimitExceeded { .. },
            ))
        );
        let is_crossed = matches!(
            loaded,
            Err(campaign_choregraph::AdapterError::Campaign(
                campaign_core::CampaignError::CrossedIdentity,
            ))
        );
        let is_journalled = (is_exceeded && self.opens(super::Hole::HeldOnProjectionLimit))
            || (is_crossed && self.opens(super::Hole::HeldOnCrossedIdentity));
        if is_journalled && let Some(held) = previous {
            return Ok(held.clone());
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.resyncs(request.payloads.len(), request.previous) {
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
        if self.opens(super::Hole::NonSelectionPublicationGate)
            && let (Some(publication), Some(evidence)) = (request.publication, request.evidence)
            && !matches!(
                publication.control_event(),
                campaign_core::CampaignControlEvent::SelectionPublished(_)
            )
        {
            return non_selection_gate(publication, evidence);
        }
        self.inner.execution_gate(request)
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
