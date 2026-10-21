/// Validates one derived result against the state exactly as replay does; returns the index of
/// the outstanding selection it settles.
// r[impl campaign.observations]
// r[impl campaign.observations.duplicate]
pub(crate) fn validate_observation(
    state: &crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
) -> crate::error::CampaignResult<usize> {
    if observation.campaign_id != state.admission.campaign_id
        || observation.adapter_id != state.admission.adapter_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    observation
        .payload_id
        .require_kind(crate::identity::IdKind::EventPayload)?;
    let outstanding_index =
        outstanding_position(state, observation.selection_id, observation.result_id)?;
    let outstanding = &state.outstanding[outstanding_index];
    if outstanding.parent_moment_id != observation.parent_moment_id
        || outstanding.operation_id != observation.operation_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    crate::frontier::validation::validate_children(state, observation)?;
    crate::frontier::validation::validate_findings(state, &observation.findings)?;
    validate_derived_records(state, &observation.derived_records)?;
    check_campaign_findings(state, &observation.findings)?;
    check_payload_conflicts(state, observation)?;
    // An honest state lists an accepted result only with its settled selection, which
    // `outstanding_position` refuses first; a caller-built state may list it alone.
    if state
        .accepted_results
        .binary_search(&observation.result_id)
        .is_ok()
    {
        return Err(crate::error::CampaignError::DuplicateResult);
    }
    debug_assert_eq!(outstanding.selection_id, observation.selection_id);
    debug_assert_eq!(outstanding.operation_id, observation.operation_id);
    Ok(outstanding_index)
}

/// Validates one unobserved closure against the state exactly as replay does; returns the index
/// of the outstanding selection it settles.
// r[impl campaign.unobserved]
// r[impl campaign.observations.duplicate]
pub(crate) fn validate_closure(
    state: &crate::frontier::FrontierState,
    closure: &crate::model::UnobservedClosure,
) -> crate::error::CampaignResult<usize> {
    if closure.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if !closure.uncertainty_acknowledged {
        return Err(crate::error::CampaignError::MissingUncertaintyAcknowledgement);
    }
    closure
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    if crate::model::derive_closure_id(closure)? != closure.closure_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let outstanding_index = outstanding_position(state, closure.selection_id, closure.closure_id)?;
    debug_assert!(closure.uncertainty_acknowledged);
    debug_assert!(outstanding_index < state.outstanding.len());
    Ok(outstanding_index)
}

/// Finds the outstanding selection a settlement closes.
///
/// A settled selection accepts no second settlement: the identical settlement is a
/// `DuplicateResult`, and any other one would introduce new exploration through a repeated
/// execution of one published selection (`InvalidRetryBoundary`).
// r[impl campaign.observations.duplicate]
// r[impl campaign.retry_boundary]
fn outstanding_position(
    state: &crate::frontier::FrontierState,
    selection_id: crate::identity::Id,
    settlement_id: crate::identity::Id,
) -> crate::error::CampaignResult<usize> {
    if let Some(index) = crate::identity::collection::sorted_position(
        &state.outstanding,
        selection_id,
        |selection| selection.selection_id,
    ) {
        return Ok(index);
    }
    let settled =
        crate::identity::collection::sorted_position(&state.settled, selection_id, |settlement| {
            settlement.selection_id
        })
        .map(|index| &state.settled[index])
        .ok_or(crate::error::CampaignError::MissingSelection)?;
    if settled.settlement_id == settlement_id {
        return Err(crate::error::CampaignError::DuplicateResult);
    }
    Err(crate::error::CampaignError::InvalidRetryBoundary)
}

fn validate_derived_records(
    state: &crate::frontier::FrontierState,
    records: &[crate::identity::Id],
) -> crate::error::CampaignResult<()> {
    crate::model::check_len(records.len(), state.admission.bounds.max_derived_records)?;
    for record in records {
        record.require_kind(crate::identity::IdKind::DerivedRecord)?;
    }
    let sorted = crate::identity::sorted_copy(records);
    if !crate::identity::unique_sorted_ids(&sorted) {
        return Err(crate::error::CampaignError::DuplicateIdentity);
    }
    debug_assert_eq!(sorted.len(), records.len());
    Ok(())
}

/// Checks the campaign finding total; a re-reported finding must repeat its accepted source facts.
// r[impl campaign.findings]
fn check_campaign_findings(
    state: &crate::frontier::FrontierState,
    findings: &[crate::model::FindingRef],
) -> crate::error::CampaignResult<()> {
    let mut new_findings: usize = 0;
    for finding in findings {
        match crate::identity::collection::sorted_position(
            &state.findings,
            finding.finding_id,
            |existing| existing.finding_id,
        )
        .map(|index| &state.findings[index])
        {
            None => {
                new_findings = new_findings
                    .checked_add(1)
                    .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
            }
            Some(existing) if existing != finding => {
                return Err(crate::error::CampaignError::ConflictingResult);
            }
            Some(_) => {}
        }
    }
    let total = state
        .findings
        .len()
        .checked_add(new_findings)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    crate::model::check_len(total, state.admission.bounds.max_findings)?;
    debug_assert!(new_findings <= findings.len());
    Ok(())
}

/// Rejects a result whose payload is already bound, and a child whose exploration payload is
/// already bound to another child moment or result, in the state or within the result.
///
/// Rediscovering an existing child moment is not a conflict: it observes the first discovery, and
/// its payload is bound to that child.
// r[impl campaign.fork.duplicate]
fn check_payload_conflicts(
    state: &crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
) -> crate::error::CampaignResult<()> {
    if state.payload_owner(observation.payload_id).is_some() {
        return Err(crate::error::CampaignError::ConflictingResult);
    }
    let mut child_payloads = alloc::vec::Vec::with_capacity(observation.children.len());
    for child in &observation.children {
        let is_claimed_elsewhere = state
            .payload_owner(child.event_payload_id)
            .is_some_and(|owner| owner != child.child_moment_id);
        let is_result_payload = child.event_payload_id == observation.payload_id;
        if is_claimed_elsewhere || is_result_payload {
            return Err(crate::error::CampaignError::ConflictingResult);
        }
        child_payloads.push(child.event_payload_id);
    }
    child_payloads.sort();
    if !crate::identity::unique_sorted_ids(&child_payloads) {
        return Err(crate::error::CampaignError::ConflictingResult);
    }
    debug_assert!(state.payload_owner(observation.payload_id).is_none());
    debug_assert!(
        observation
            .children
            .iter()
            .all(|child| child.event_payload_id != observation.payload_id)
    );
    Ok(())
}
