//! Holed shell answers: partial own gates, residual-denial journals, and a replaying cache check.

/// Whether a cache denial belongs to a class the cache oracle once never presented.
pub(super) fn is_unpresented_cache_denial(error: &campaign_choregraph::AdapterError) -> bool {
    match error {
        campaign_choregraph::AdapterError::Campaign(error) => {
            *error != campaign_core::CampaignError::InvalidCache
        }
        other => matches!(
            other,
            campaign_choregraph::AdapterError::PayloadIdentityMismatch
                | campaign_choregraph::AdapterError::MetadataMismatch
                | campaign_choregraph::AdapterError::ParentLineageMismatch
                | campaign_choregraph::AdapterError::NonApplicationEnvelope
                | campaign_choregraph::AdapterError::SchemaIdentityMismatch
                | campaign_choregraph::AdapterError::ProjectionIdentityMismatch
                | campaign_choregraph::AdapterError::History(_)
        ),
    }
}

/// Whether a load denial belongs to a class the load matrix once never presented.
pub(super) fn is_residual_load_denial(error: &campaign_choregraph::AdapterError) -> bool {
    match error {
        campaign_choregraph::AdapterError::Campaign(error) => {
            *error != campaign_core::CampaignError::StaleState
        }
        campaign_choregraph::AdapterError::History(error) => !matches!(
            error,
            campaign_choregraph::HistoryError::AmbiguousLineage { .. }
        ),
        other => matches!(
            other,
            campaign_choregraph::AdapterError::MetadataMismatch
                | campaign_choregraph::AdapterError::NonApplicationEnvelope
                | campaign_choregraph::AdapterError::SchemaIdentityMismatch
                | campaign_choregraph::AdapterError::ProjectionIdentityMismatch
        ),
    }
}

/// Validates a cache against core replay of the presented payloads under the fresh binding.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the binding, the replay, or the cache.
pub(super) fn replayed_cache(
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
    campaign_core::validate_cache_for_state(cache, &fresh, &state)?;
    Ok(state)
}

/// The decision a presented publication published, if it publishes a selection.
pub(super) fn published_decision<'r>(
    request: &crate::port::ExecutionGateRequest<'r>,
) -> Option<&'r campaign_core::SelectionDecision> {
    match request.publication?.control_event() {
        campaign_core::CampaignControlEvent::SelectionPublished(event) => Some(&event.decision),
        _ => None,
    }
}

/// Whether `presented` has the published selection identity, candidate, operation, and ordinal.
pub(super) fn has_execution_fields(
    published: &campaign_core::SelectionDecision,
    presented: &campaign_core::SelectionDecision,
) -> bool {
    published.selection_id == presented.selection_id
        && published.candidate_id == presented.candidate_id
        && published.operation_id == presented.operation_id
        && published.ordinal == presented.ordinal
}

/// Authority of one own gate decision.
const fn authority(is_granted: bool) -> crate::port::ExecutionAuthority {
    if is_granted {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}

/// Own gate over durability, the exact published decision, the evidence's event, and the before
/// and after heads.
pub(super) fn head_only_gate(
    request: &crate::port::ExecutionGateRequest<'_>,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence), Some(published)) = (
        request.publication,
        request.evidence,
        published_decision(request),
    ) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    authority(
        evidence.durable_event
            && evidence.durable_branch_update
            && *published == *request.selection
            && evidence.history_event_id == publication.history_event_id()
            && evidence.branch_before.head == publication.branch_plan().before.head
            && evidence.branch_after.head == publication.history_event_id(),
    )
}

/// Own gate over durability, the exact published decision, the planned branch value before, and
/// the evidence's internal consistency.
pub(super) fn self_consistent_gate(
    request: &crate::port::ExecutionGateRequest<'_>,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence), Some(published)) = (
        request.publication,
        request.evidence,
        published_decision(request),
    ) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    let before = &publication.branch_plan().before;
    authority(
        evidence.durable_event
            && evidence.durable_branch_update
            && *published == *request.selection
            && evidence.branch_before == *before
            && evidence.branch_after.name == before.name
            && evidence.branch_after.head == evidence.history_event_id
            && before.generation.checked_add(1) == Some(evidence.branch_after.generation),
    )
}
