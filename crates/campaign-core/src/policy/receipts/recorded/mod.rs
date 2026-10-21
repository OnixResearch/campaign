//! Receipts of recorded control events and of explicit admission refusals.

/// Projects the receipt of one accepted control event.
///
/// The request's profile must be a valid production profile for the state (a reference-evidence
/// profile is refused). The event is authenticated first: its supplied identity must be an
/// accepted event of the state, and the presented event must equal its canonical re-derivation
/// (`CampaignControlEvent::canonical`: every carried derived identity re-derived, every canonical
/// list in canonical order), as replay requires. For a candidate batch the receipt binds the
/// event's admission-outcome list identity (so a coalesced duplicate is recorded with the
/// candidate it was coalesced into) and the admission and coalescing policies that decided the
/// batch. For a guidance event it binds the outcome identity of the recorded dynamic. For a
/// selection event the request's profile must be the profile the event carries, and the receipt
/// binds the event's entropy ticket, guidance record and cost reservation. Other events bind no
/// outcome list.
///
/// # Errors
///
/// Returns `StaleState` when the state is not current, `MissingAdmission` for a state without its
/// admission event (zero control events), `ReferencePolicySelected` for a
/// reference-evidence profile, `MalformedIdentity` when the event differs from its canonical
/// re-derivation, `CrossedIdentity` when the event is not an accepted event of the state, a
/// selection event carries another profile than the request's, or the binding names another
/// history, and typed identity errors for malformed request or profile identities.
// r[impl campaign.receipts]
// r[impl campaign.receipts.crossed]
pub fn project_event_receipt(
    state: &crate::frontier::FrontierState,
    request: &crate::model::EventReceiptRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    crate::policy::validation::assert_state_identity_current(state)?;
    super::require_state_history(state, current_projection)?;
    request
        .request_id
        .require_kind(crate::identity::IdKind::Request)?;
    crate::policy::decision::checks::validate_profile(state, &request.profile)?;
    let event_id = request
        .event
        .event_id()
        .require_kind(crate::identity::IdKind::Event)?;
    if state.event_ids.binary_search(&event_id).is_err() {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if request.event.canonical()? != request.event {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let mut receipt = super::base_receipt(
        state,
        current_projection,
        request.request_id,
        &request.profile,
        event_id,
    )?;
    receipt = bind_event_outcomes(receipt, &request.event, &request.profile)?;
    debug_assert_eq!(receipt.decision_id, event_id);
    debug_assert_eq!(receipt.state_id, state.state_id);
    super::finish(receipt)
}

/// Projects the receipt of one candidate batch refused by its declared `Reject` rule, recording
/// the rejection reason.
///
/// The refusal must be exactly the one `explain_admission_refusal` returns for its submission on
/// `state`, and the request's profile must be a valid production profile for the state. The
/// receipt binds the refusal identity as its decision and the refusal's own admission and
/// coalescing policies.
///
/// # Errors
///
/// Returns `StaleState` when the state's identity is not its canonical one, `MissingAdmission` for a
/// state without its admission event (zero control events), `CrossedIdentity` when the binding
/// names another history or the refusal is not the one the rule produces for the state,
/// `ReferencePolicySelected` for a reference-evidence profile, and every typed error
/// `explain_admission_refusal` returns.
// r[impl campaign.fd.admission.policy]
// r[impl campaign.receipts.crossed]
pub fn project_refusal_receipt(
    state: &crate::frontier::FrontierState,
    request: &crate::model::RefusalReceiptRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    crate::policy::validation::assert_state_identity_current(state)?;
    super::require_state_history(state, current_projection)?;
    request
        .request_id
        .require_kind(crate::identity::IdKind::Request)?;
    crate::policy::decision::checks::validate_profile(state, &request.profile)?;
    let refusal = &request.refusal;
    let explained = crate::policy::intake::explain_admission_refusal(
        state,
        refusal.candidates.clone(),
        refusal.admission_rule,
        refusal.coalescing_policy_id,
    )?;
    if explained.as_ref() != Some(refusal) {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let mut receipt = super::base_receipt(
        state,
        current_projection,
        request.request_id,
        &request.profile,
        refusal.refusal_id,
    )?;
    receipt.admission_policy_id = refusal.admission_policy_id;
    receipt.coalescing_policy_id = refusal.coalescing_policy_id;
    receipt.admission_refusal = Some(refusal.reason);
    debug_assert_eq!(receipt.state_id, refusal.state_id);
    super::finish(receipt)
}

/// Binds the facts of an authenticated (canonical) event.
// r[impl campaign.fd.admission]
// r[impl campaign.receipts.crossed]
fn bind_event_outcomes(
    mut receipt: crate::model::CampaignReceipt,
    event: &crate::model::CampaignControlEvent,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    match event {
        crate::model::CampaignControlEvent::CandidatesAdmitted(batch) => {
            receipt.admission_outcomes_id = Some(batch.admission_outcomes_id);
            receipt.admission_policy_id = batch.admission_policy_id;
            receipt.coalescing_policy_id = batch.coalescing_policy_id;
        }
        crate::model::CampaignControlEvent::GuidanceAdmitted(guidance) => {
            receipt.admission_outcomes_id = Some(guidance.admission_outcomes_id);
        }
        crate::model::CampaignControlEvent::SelectionPublished(selection) => {
            if *selection.profile != *profile {
                return Err(crate::error::CampaignError::CrossedIdentity);
            }
            let decision = &selection.decision;
            receipt.entropy_ticket_id = decision.entropy_id;
            receipt.guidance_record_id = Some(decision.guidance_id);
            receipt.cost_reservation_id = Some(crate::model::derive_cost_reservation_id(decision)?);
            debug_assert_eq!(receipt.selection_profile_id, decision.profile_id);
        }
        crate::model::CampaignControlEvent::CampaignAdmitted { .. }
        | crate::model::CampaignControlEvent::ResultObserved(_)
        | crate::model::CampaignControlEvent::UnobservedClosed(_)
        | crate::model::CampaignControlEvent::CandidateRetired(_)
        | crate::model::CampaignControlEvent::CandidatesPruned(_)
        | crate::model::CampaignControlEvent::StopAccepted(_) => {}
    }
    debug_assert!(
        receipt
            .admission_outcomes_id
            .is_none_or(|id| id.kind == crate::identity::IdKind::AdmissionOutcomes)
    );
    Ok(receipt)
}
