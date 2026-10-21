//! Shell points of the re-audit wrapper.

const fn authority(is_granted: bool) -> crate::port::ExecutionAuthority {
    if is_granted {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}

/// Own gate: durable, self-consistent evidence of a selection event whose selection identity
/// equals the requested one; the decision's other fields are never compared.
fn id_only_gate(
    request: &crate::port::ExecutionGateRequest<'_>,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence)) = (request.publication, request.evidence) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    let campaign_core::CampaignControlEvent::SelectionPublished(event) =
        publication.control_event()
    else {
        return crate::port::ExecutionAuthority::Refused;
    };
    authority(
        evidence.durable_event
            && evidence.durable_branch_update
            && evidence.history_event_id == publication.history_event_id()
            && evidence.branch_before == publication.branch_plan().before
            && evidence.branch_after == publication.branch_plan().after
            && event.decision.selection_id == request.selection.selection_id,
    )
}

/// Cache check by fresh binding and reconstructed state identity only.
fn binding_state_cache(
    request: &campaign_choregraph::CacheValidationRequest<'_>,
    cache: &campaign_core::CacheProjection,
) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
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
    if cache.binding != fresh || cache.state_id != state.state_id {
        return Err(campaign_core::CampaignError::InvalidCache.into());
    }
    Ok(state)
}

/// Cache check by projection identity and presented lineage only.
fn projection_id_cache(
    request: &campaign_choregraph::CacheValidationRequest<'_>,
    cache: &campaign_core::CacheProjection,
) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
    let fresh = campaign_choregraph::derive_projection_binding(
        request.graph,
        request.branch,
        request.projection_schema_id,
        request.adapter_id,
        request.control_domain_id,
    )?;
    if cache.binding.projection_id != fresh.projection_id {
        return Err(campaign_core::CampaignError::InvalidCache.into());
    }
    debug_assert_eq!(cache.binding.projection_id, fresh.projection_id);
    let validated = campaign_choregraph::validate_control_projection(
        &campaign_choregraph::ControlProjectionRequest {
            graph: request.graph,
            branch: request.branch,
            parent_choices: request.parent_choices,
            maximum_events: request.maximum_events,
            expected_binding: fresh.clone(),
            payloads: request.payloads,
        },
    )?;
    debug_assert_eq!(*validated.binding(), fresh);
    Ok(validated.state().clone())
}

impl crate::port::ConsumerShell for super::HoledAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.opens(super::Hole::ResultEventOverclaim) {
            for payload in request.payloads {
                if let campaign_core::CampaignControlEvent::ResultObserved(result) =
                    &payload.control_event
                    && !self.result_events.contains(&result.event_id)
                {
                    self.result_events.push(result.event_id);
                }
            }
        }
        let loaded = self.inner.load_history(request, previous);
        if !self.opens(super::Hole::JournalOnUnpresentedDenials) {
            return loaded;
        }
        match loaded {
            Ok(projection) => {
                self.journal = Some(projection.clone());
                Ok(projection)
            }
            Err(error) => {
                let is_unpresented = matches!(
                    error,
                    campaign_choregraph::AdapterError::ProjectionPayloadMismatch
                        | campaign_choregraph::AdapterError::NonFastForwardBranch
                        | campaign_choregraph::AdapterError::ControlBranchMismatch
                );
                match (is_unpresented, self.journal.clone()) {
                    (true, Some(journal)) => Ok(journal),
                    _ => Err(error),
                }
            }
        }
    }

    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.opens(super::Hole::BindingStateCache) {
            return binding_state_cache(request, cache);
        }
        if self.opens(super::Hole::ProjectionIdOnlyCache) {
            return projection_id_cache(request, cache);
        }
        self.inner.validate_cache(request, cache)
    }

    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        if self.opens(super::Hole::IdOnlyGate) {
            return id_only_gate(request);
        }
        if self.opens(super::Hole::ExploratoryGateBypass) && request.selection.entropy_id.is_some()
        {
            return crate::port::ExecutionAuthority::Granted;
        }
        if self.opens(super::Hole::EvidenceOptionalGate)
            && let (Some(publication), None) = (request.publication, request.evidence)
        {
            return authority(matches!(
                publication.control_event(),
                campaign_core::CampaignControlEvent::SelectionPublished(event)
                    if event.decision == *request.selection
            ));
        }
        self.inner.execution_gate(request)
    }

    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.opens(super::Hole::QuietStopClaim)
            && receipt.decision_id.kind != campaign_core::IdKind::Selection
            && !signals.is_selected
        {
            claims.exhaustive_search = true;
            claims.safe_deletion = true;
        }
        if self.opens(super::Hole::ExploratoryReceiptOverclaim)
            && receipt.entropy_ticket_id.is_some()
        {
            claims.exhaustive_search = true;
        }
        if self.opens(super::Hole::MixedSignalOverclaim) {
            claims.operation_success = signals.has_zero_exit_status && !signals.has_check_reference;
        }
        if self.opens(super::Hole::ResultEventOverclaim)
            && self.result_events.contains(&receipt.decision_id)
        {
            claims.operation_success = true;
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
        if self.opens(super::Hole::ExploratoryRoleCollapse) && work_item.entropy_id.is_some() {
            roles.queued_work_id = candidate.candidate_id;
            roles.computed_result_id = candidate.candidate_id;
        }
        roles
    }
}
