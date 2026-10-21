//! Consumer shell points every conforming fixture adapter delegates to Campaign.

/// Loads a control projection only through Choregraph validation: as a fast-forward of the
/// previously validated projection when the host holds one, otherwise as a cold start.
///
/// # Errors
///
/// Returns the adapter error of a stale, rewound, forked, renamed, lagging, rewritten, forged, or
/// ambiguous history.
pub(super) fn load_history(
    request: &campaign_choregraph::ControlProjectionRequest<'_>,
    previous: Option<&campaign_choregraph::ValidatedControlProjection>,
) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
    match previous {
        Some(previous) => campaign_choregraph::validate_control_advance(previous, request),
        None => campaign_choregraph::validate_control_projection(request),
    }
}

/// Validates a derived cache only against authoritative history, as a fast-forward of the
/// request's held projection when it carries one.
///
/// # Errors
///
/// Returns the adapter error of a stale or incomplete cache.
pub(super) fn validate_cache(
    request: &campaign_choregraph::CacheValidationRequest<'_>,
    cache: &campaign_core::CacheProjection,
) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
    campaign_choregraph::validate_cache_against_history(request, cache)
}

/// Grants execution only when Campaign's publication gate accepts the durable publication.
pub(super) fn execution_gate(
    request: &crate::port::ExecutionGateRequest<'_>,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence)) = (request.publication, request.evidence) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    let is_granted = campaign_choregraph::can_execute_selection_after_publication(
        request.selection,
        publication,
        evidence,
    ) == Ok(true);
    if is_granted {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}

/// Campaign receipts carry no promotion authority; consumer signals never become claims.
pub(super) fn promotion_claims() -> campaign_core::ReceiptClaims {
    campaign_core::ReceiptClaims::no_claims()
}

/// Distinct records keep distinct identities: candidate, selection, and result.
pub(super) const fn role_identities(
    candidate: &campaign_core::CandidateRecord,
    work_item: &campaign_core::SelectionDecision,
    result: &campaign_core::ResultObservation,
) -> campaign_core::RoleCollapseFixture {
    campaign_core::RoleCollapseFixture {
        candidate_record_id: candidate.candidate_id,
        queued_work_id: work_item.selection_id,
        computed_result_id: result.result_id,
    }
}

/// Entropy ticket whose bytes carry the explore draw (`0..8`) and index draw (`8..16`).
///
/// # Errors
///
/// Returns a typed Campaign error when ticket derivation fails.
pub(super) fn entropy_ticket(
    source_id: campaign_core::Id,
    draw: &crate::vocabulary::ScenarioDraw,
    context: &crate::port::EntropyContext,
) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
    let mut bytes = [0u8; campaign_core::DIGEST_LENGTH];
    let (explore, rest) = bytes.split_at_mut(campaign_core::ENTROPY_PREFIX_BYTES);
    explore.copy_from_slice(&draw.explore_draw.to_le_bytes());
    let index = rest
        .get_mut(..campaign_core::ENTROPY_PREFIX_BYTES)
        .ok_or(campaign_core::CampaignError::InvalidEntropy)?;
    index.copy_from_slice(&draw.index_draw.to_le_bytes());
    Ok(campaign_core::EntropyTicket {
        entropy_id: campaign_core::Id::absent(),
        campaign_id: context.campaign_id,
        state_id: context.state_id,
        profile_id: context.selection_profile_id,
        source_id,
        counter: context.counter,
        bytes,
    }
    .derive()?)
}

/// Acknowledged unobserved closure with the consumer's reason.
///
/// # Errors
///
/// Returns a typed Campaign error when identity derivation fails.
pub(super) fn closure(
    context: &crate::port::ClosureContext<'_>,
    reason_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
    let mut closure = campaign_core::UnobservedClosure {
        closure_id: campaign_core::Id::absent(),
        campaign_id: context.campaign_id,
        selection_id: context.selection.selection_id,
        reason_id,
        uncertainty_acknowledged: true,
    };
    closure.closure_id = campaign_core::derive_closure_id(&closure)?;
    Ok(closure)
}

/// Declared costs of an operation in the adapter's resource dimensions.
///
/// # Errors
///
/// Returns `InvalidIdentities` for an undeclared dimension.
pub(super) fn declared_costs(
    operation: &crate::vocabulary::ScenarioOperation,
    dimensions: &[campaign_core::Id],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::ResourceAmount>> {
    let mut costs = alloc::vec::Vec::with_capacity(operation.costs.len());
    for cost in &operation.costs {
        let index = usize::try_from(cost.dimension)
            .map_err(|_| crate::verdicts::ConformanceError::InvalidIdentities)?;
        let dimension_id = dimensions
            .get(index)
            .copied()
            .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?;
        costs.push(campaign_core::ResourceAmount {
            dimension_id,
            amount: cost.units,
        });
    }
    Ok(costs)
}
