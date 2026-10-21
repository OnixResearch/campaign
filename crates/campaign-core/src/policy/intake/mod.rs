mod ranks;
pub use ranks::{admit_guidance, revise_guidance};
/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
// r[impl campaign.initial_admission]
pub fn admit_campaign(
    input: crate::model::CampaignAdmission,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    let admission = input.derive()?;
    let event_id = crate::model::derive_campaign_event_id(&admission)?;
    Ok(crate::model::CampaignControlEvent::CampaignAdmitted {
        event_id,
        admission,
    })
}
/// Admits one candidate batch under the declared admission rule.
///
/// Candidates are canonically ordered by identity. Under `DuplicateCandidateRule::Reject` any
/// duplicate, within the batch or against the state, denies the whole batch (see
/// `explain_admission_refusal` for the refusal value); under `Coalesce` duplicates are removed and
/// recorded as `CoalescedDuplicate` outcomes.
///
/// # Errors
///
/// Returns `DuplicateIdentity` when the `Reject` rule refuses the batch, `ControlEventLimitReached`
/// without control-event headroom, and a typed `CampaignError` when explicit values are missing,
/// crossed, stale, malformed, over bounds, or would violate policy invariants.
// r[impl campaign.candidate_admission]
pub fn admit_candidates(
    state: &crate::frontier::FrontierState,
    candidates: alloc::vec::Vec<crate::model::CandidateRecord>,
    admission_rule: crate::model::AdmissionRule,
    coalescing_policy_id: crate::identity::Id,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    decide_batch(state, candidates, admission_rule, coalescing_policy_id)?
        .map_err(|_refusal| crate::error::CampaignError::DuplicateIdentity)
}

/// Explains the refusal `admit_candidates` returns for the same submission: the declared `Reject`
/// rule's rejection reason as an explicit value that `project_refusal_receipt` can record.
///
/// Returns `None` when the rule does not refuse the batch (the batch is admissible). The state is
/// never changed.
///
/// # Errors
///
/// Returns every typed error `admit_candidates` returns for the submission other than the
/// `DuplicateIdentity` refusal itself.
pub fn explain_admission_refusal(
    state: &crate::frontier::FrontierState,
    candidates: alloc::vec::Vec<crate::model::CandidateRecord>,
    admission_rule: crate::model::AdmissionRule,
    coalescing_policy_id: crate::identity::Id,
) -> crate::error::CampaignResult<Option<crate::model::AdmissionRefusal>> {
    Ok(decide_batch(state, candidates, admission_rule, coalescing_policy_id)?.err())
}

/// Decides one submission: the admitted batch event, or the `Reject` rule's refusal.
// r[impl campaign.candidate_admission.invalid]
// r[impl campaign.candidate_admission.coalesce]
fn decide_batch(
    state: &crate::frontier::FrontierState,
    mut candidates: alloc::vec::Vec<crate::model::CandidateRecord>,
    admission_rule: crate::model::AdmissionRule,
    coalescing_policy_id: crate::identity::Id,
) -> crate::error::CampaignResult<
    Result<crate::model::CampaignControlEvent, crate::model::AdmissionRefusal>,
> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::policy::validation::ensure_open_for_new_work(state)?;
    let admission_policy_id = admission_rule.admission_policy_id()?;
    coalescing_policy_id.require_kind(crate::identity::IdKind::CoalescingPolicy)?;
    crate::model::check_len_at_least_one(
        candidates.len(),
        state.admission.bounds.max_candidate_batch,
        crate::error::CampaignError::MissingCandidate,
    )?;
    for candidate in &mut candidates {
        if candidate.candidate_id.is_absent() {
            *candidate = candidate.clone().derive()?;
        }
        crate::frontier::validation::validate_candidate_admission(state, candidate)?;
    }
    candidates.sort_by_key(|candidate| candidate.candidate_id);
    let submitted: alloc::vec::Vec<crate::identity::Id> = candidates
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect();
    let outcomes =
        match crate::frontier::validation::rule_outcome(state, &submitted, admission_rule)? {
            Ok(outcomes) => outcomes,
            Err(reason) => {
                let refusal = crate::model::AdmissionRefusal {
                    refusal_id: crate::identity::Id::absent(),
                    campaign_id: state.admission.campaign_id,
                    state_id: state.state_id,
                    candidates,
                    admission_rule,
                    admission_policy_id,
                    coalescing_policy_id,
                    reason,
                }
                .derive()?;
                debug_assert_eq!(refusal.state_id, state.state_id);
                return Ok(Err(refusal));
            }
        };
    let admitted: alloc::vec::Vec<crate::model::CandidateRecord> = candidates
        .into_iter()
        .zip(outcomes.iter())
        .filter(|(_, outcome)| outcome.outcome == crate::model::AdmissionDecision::Admitted)
        .map(|(candidate, _)| candidate)
        .collect();
    let total = state
        .candidates
        .len()
        .checked_add(admitted.len())
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    crate::model::check_len(total, state.admission.bounds.max_candidates)?;
    debug_assert!(admitted.len() <= outcomes.len());
    let mut batch = crate::model::CandidateBatchEvent {
        event_id: crate::identity::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidates: admitted,
        admission_rule,
        admission_policy_id,
        coalescing_policy_id,
        admission_outcomes_id: crate::model::derive_admission_outcomes_id(&outcomes)?,
        outcomes,
    };
    batch.event_id = batch.expected_event_id()?;
    Ok(Ok(crate::model::CampaignControlEvent::CandidatesAdmitted(
        batch,
    )))
}
