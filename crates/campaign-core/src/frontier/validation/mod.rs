/// Checks the admitted schema, depth bound, known parent, and exact parent depth of one candidate.
// r[impl campaign.candidate_admission.invalid]
pub(crate) fn validate_candidate_lineage(
    state: &crate::frontier::FrontierState,
    candidate: &crate::model::CandidateRecord,
) -> crate::error::CampaignResult<()> {
    if !state
        .admission
        .supports_operation_schema(candidate.operation_schema_id)
    {
        return Err(crate::error::CampaignError::UnsupportedSchema);
    }
    if candidate.depth > state.admission.bounds.max_depth {
        return Err(crate::error::CampaignError::CandidateOverDepth);
    }
    let parent_depth = state
        .known_moment_depth(candidate.parent_moment_id)
        .ok_or(crate::error::CampaignError::MissingParentMoment)?;
    let expected_depth = parent_depth
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    if candidate.depth != expected_depth {
        return Err(crate::error::CampaignError::InvalidDepth);
    }
    debug_assert!(candidate.depth > parent_depth);
    debug_assert!(candidate.depth <= state.admission.bounds.max_depth);
    Ok(())
}
/// Checks every intake and replay condition of one derived candidate against the state.
pub(crate) fn validate_candidate_admission(
    state: &crate::frontier::FrontierState,
    candidate: &crate::model::CandidateRecord,
) -> crate::error::CampaignResult<()> {
    candidate.validate(state.admission.campaign_id)?;
    if candidate.source_adapter_id != state.admission.adapter_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    crate::frontier::validation::validate_candidate_lineage(state, candidate)?;
    crate::model::check_len(
        candidate.declared_costs.len(),
        state.admission.bounds.max_resource_dimensions,
    )?;
    for cost in &candidate.declared_costs {
        let budgets = &state.admission.resource_budgets;
        let index =
            crate::identity::collection::sorted_position(budgets, cost.dimension_id, |budget| {
                budget.dimension_id
            })
            .ok_or(crate::error::CampaignError::InvalidCostProfile)?;
        if cost.amount > budgets[index].amount {
            return Err(crate::error::CampaignError::BudgetExhausted);
        }
    }
    debug_assert!(candidate.depth <= state.admission.bounds.max_depth);
    debug_assert!(state.has_known_moment(candidate.parent_moment_id));
    Ok(())
}
/// Applies the declared duplicate rule to canonically sorted submitted candidate identities.
///
/// The inner result is the per-candidate outcome list, or the `Reject` rule's refusal reason.
pub(crate) fn rule_outcome(
    state: &crate::frontier::FrontierState,
    sorted_ids: &[crate::identity::Id],
    rule: crate::model::AdmissionRule,
) -> crate::error::CampaignResult<
    Result<alloc::vec::Vec<crate::model::AdmissionOutcome>, crate::model::AdmissionRefusalReason>,
> {
    if !sorted_ids.is_sorted() {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let mut outcomes = alloc::vec::Vec::with_capacity(sorted_ids.len());
    let mut previous: Option<crate::identity::Id> = None;
    for candidate_id in sorted_ids {
        let is_in_batch = previous == Some(*candidate_id);
        let is_admitted = state.candidate(*candidate_id).is_ok();
        let outcome = match (is_in_batch || is_admitted, rule.duplicate_candidate) {
            (false, _) => crate::model::AdmissionDecision::Admitted,
            (true, crate::model::DuplicateCandidateRule::Reject) => {
                let duplicate_of = if is_admitted {
                    crate::model::DuplicateOrigin::AdmittedCandidate
                } else {
                    crate::model::DuplicateOrigin::SameBatch
                };
                return Ok(Err(crate::model::AdmissionRefusalReason {
                    candidate_id: *candidate_id,
                    duplicate_of,
                }));
            }
            (true, crate::model::DuplicateCandidateRule::Coalesce) => {
                crate::model::AdmissionDecision::CoalescedDuplicate {
                    existing_candidate_id: *candidate_id,
                }
            }
        };
        outcomes.push(crate::model::AdmissionOutcome {
            candidate_id: *candidate_id,
            outcome,
        });
        previous = Some(*candidate_id);
    }
    debug_assert_eq!(outcomes.len(), sorted_ids.len());
    Ok(Ok(outcomes))
}
/// Applies the declared duplicate rule; a `Reject` refusal is `DuplicateIdentity`.
pub(crate) fn candidate_outcomes(
    state: &crate::frontier::FrontierState,
    sorted_ids: &[crate::identity::Id],
    rule: crate::model::AdmissionRule,
) -> crate::error::CampaignResult<alloc::vec::Vec<crate::model::AdmissionOutcome>> {
    rule_outcome(state, sorted_ids, rule)?
        .map_err(|_refusal| crate::error::CampaignError::DuplicateIdentity)
}
/// Checks the guidance rank shape, profile, adapter, and retained candidate; returns the candidate
/// index.
///
/// The rank-vector length is bounded by `max_rank_dimensions` and, once any guidance record is
/// admitted, must equal the campaign's rank shape (`InvalidGuidanceShape` otherwise), so every
/// admitted rank vector is comparable under one total lexicographic order.
// r[impl campaign.guidance.invalid]
// r[impl campaign.guidance.shape]
pub(crate) fn validate_guidance_shape(
    state: &crate::frontier::FrontierState,
    guidance: &crate::model::GuidanceRecord,
) -> crate::error::CampaignResult<usize> {
    guidance.validate(state.admission.campaign_id)?;
    crate::model::check_len(
        guidance.ranks.len(),
        state.admission.bounds.max_rank_dimensions,
    )?;
    let max_magnitude = state.admission.bounds.max_rank_magnitude;
    if guidance
        .ranks
        .iter()
        .any(|rank| rank.unsigned_abs() > max_magnitude)
    {
        return Err(crate::error::CampaignError::InvalidGuidanceShape);
    }
    if crate::frontier::validation::guidance_rank_dimensions(state)
        .is_some_and(|dimensions| dimensions != guidance.ranks.len())
    {
        return Err(crate::error::CampaignError::InvalidGuidanceShape);
    }
    if guidance.ordering_profile_id != state.admission.guidance_profile_id
        || guidance.adapter_id != state.admission.adapter_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let index = crate::frontier::validation::find_candidate_index(state, guidance.candidate_id)?;
    crate::frontier::dispatch::ensure_candidate_selectable(&state.candidates[index])?;
    debug_assert!(index < state.candidates.len());
    Ok(index)
}
/// The campaign's guidance rank-vector length, fixed by its first admitted guidance record.
///
/// Candidates are never removed and keep their latest record, and every admitted record has the
/// same length, so any candidate's latest record carries the length the first record fixed.
pub(crate) fn guidance_rank_dimensions(state: &crate::frontier::FrontierState) -> Option<usize> {
    state
        .candidates
        .iter()
        .find_map(|candidate| candidate.latest_guidance.as_ref())
        .map(|guidance| guidance.ranks.len())
}
/// Returns the guidance record count after one more record, within the declared bound.
pub(crate) fn next_guidance_record_count(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<u64> {
    let next = state
        .guidance_record_count
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    if next > state.admission.bounds.max_guidance_records {
        return Err(crate::error::CampaignError::BoundExceeded);
    }
    Ok(next)
}
// r[impl campaign.fd.reservation.unreserved]
// r[impl campaign.observations.child_bound]
pub(super) fn validate_children(
    state: &crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
) -> crate::error::CampaignResult<()> {
    crate::model::check_len(
        observation.children.len(),
        state.admission.bounds.max_child_moments,
    )?;
    let mut child_ids = alloc::vec::Vec::with_capacity(observation.children.len());
    for child in &observation.children {
        let expected = child
            .clone()
            .derive(observation.campaign_id, observation.selection_id)?;
        if expected != *child {
            return Err(crate::error::CampaignError::InvalidChildReservation);
        }
        child_ids.push(child.child_moment_id);
    }
    child_ids.sort();
    if !crate::identity::unique_sorted_ids(&child_ids) {
        return Err(crate::error::CampaignError::DuplicateChildMoment);
    }
    debug_assert_eq!(child_ids.len(), observation.children.len());
    debug_assert!(crate::identity::unique_sorted_ids(&child_ids));
    Ok(())
}
pub(super) fn validate_findings(
    state: &crate::frontier::FrontierState,
    findings: &[crate::model::FindingRef],
) -> crate::error::CampaignResult<()> {
    crate::model::check_len(findings.len(), state.admission.bounds.max_findings)?;
    let mut ids = alloc::vec::Vec::with_capacity(findings.len());
    for finding in findings {
        finding.validate()?;
        ids.push(finding.finding_id);
    }
    ids.sort();
    if !crate::identity::unique_sorted_ids(&ids) {
        return Err(crate::error::CampaignError::InvalidFinding);
    }
    Ok(())
}
pub(crate) fn find_candidate_index(
    state: &crate::frontier::FrontierState,
    candidate_id: crate::identity::Id,
) -> crate::error::CampaignResult<usize> {
    crate::identity::collection::sorted_position(&state.candidates, candidate_id, |candidate| {
        candidate.candidate.candidate_id
    })
    .ok_or(crate::error::CampaignError::MissingCandidate)
}
