//! `campaign.fd.admission.policy`: a `Reject`-rule duplicate emits no event; its explicit refusal
//! value names the duplicate and a refusal receipt records that reason.

struct Submission {
    events: Vec<campaign_core::CampaignControlEvent>,
    state: campaign_core::FrontierState,
    existing: campaign_core::CandidateRecord,
    fresh: campaign_core::CandidateRecord,
}

fn submission() -> Result<Submission, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, existing_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"refused-existing",
        crate::suite::constants::COST_ONE,
    )?;
    let existing = crate::suite::world::observed(state.candidate(existing_id))?
        .candidate
        .clone();
    let fresh = crate::suite::world::observed(
        crate::suite::candidate::record(
            &state,
            b"refused-fresh",
            crate::suite::constants::COST_ONE,
        )
        .derive(),
    )?;
    assert_ne!(existing.candidate_id, fresh.candidate_id);
    assert!(state.candidate(fresh.candidate_id).is_err());
    Ok(Submission {
        events,
        state,
        existing,
        fresh,
    })
}

fn explain(
    state: &campaign_core::FrontierState,
    candidates: Vec<campaign_core::CandidateRecord>,
) -> Result<Option<campaign_core::AdmissionRefusal>, String> {
    crate::suite::world::observed(campaign_core::explain_admission_refusal(
        state,
        candidates,
        crate::suite::ingress::support::REJECT,
        crate::suite::ingress::support::coalescing_policy(),
    ))
}

#[test]
fn the_reject_rule_explains_its_refusal_without_an_event() -> Result<(), String> {
    let submitted = submission()?;
    let state = &submitted.state;
    let batch = vec![submitted.fresh.clone(), submitted.existing.clone()];
    assert_eq!(
        crate::suite::world::denial(crate::suite::ingress::support::admit_batch(
            state,
            batch.clone(),
            crate::suite::ingress::support::REJECT,
        ))?,
        campaign_core::CampaignError::DuplicateIdentity
    );
    let refusal = explain(state, batch)?.ok_or("the reject rule refuses a duplicate")?;
    assert_eq!(
        refusal.reason,
        campaign_core::AdmissionRefusalReason {
            candidate_id: submitted.existing.candidate_id,
            duplicate_of: campaign_core::DuplicateOrigin::AdmittedCandidate,
        }
    );
    assert_eq!(refusal.state_id, state.state_id);
    let in_batch = explain(
        state,
        vec![submitted.fresh.clone(), submitted.fresh.clone()],
    )?;
    assert_eq!(
        in_batch.map(|refused| refused.reason),
        Some(campaign_core::AdmissionRefusalReason {
            candidate_id: submitted.fresh.candidate_id,
            duplicate_of: campaign_core::DuplicateOrigin::SameBatch,
        })
    );
    assert_eq!(explain(state, vec![submitted.fresh])?, None);
    crate::suite::ingress::support::assert_unchanged(&submitted.events, state)?;
    Ok(())
}

#[test]
// r[verify campaign.fd.admission.policy]
fn a_refusal_receipt_records_the_rejection_reason() -> Result<(), String> {
    let submitted = submission()?;
    let state = &submitted.state;
    let refusal = explain(state, vec![submitted.existing.clone()])?
        .ok_or("the reject rule refuses a duplicate")?;
    let request = campaign_core::RefusalReceiptRequest {
        request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"refusal"),
        refusal: refusal.clone(),
        profile: crate::suite::selection::ranked_profile()?,
    };
    let binding = crate::suite::admission::current_projection(state)?;
    let receipt = crate::suite::world::observed(campaign_core::project_refusal_receipt(
        state, &request, &binding,
    ))?;
    assert_eq!(receipt.decision_id, refusal.refusal_id);
    assert_eq!(receipt.admission_refusal, Some(refusal.reason));
    assert_eq!(receipt.admission_policy_id, refusal.admission_policy_id);
    assert_eq!(receipt.admission_outcomes_id, None);
    crate::suite::world::observed(campaign_core::validate_receipt(
        &receipt,
        &campaign_core::ReceiptClaims::no_claims(),
    ))?;
    let mut forged = request;
    forged.refusal.reason.duplicate_of = campaign_core::DuplicateOrigin::SameBatch;
    assert_eq!(
        crate::suite::world::denial(campaign_core::project_refusal_receipt(
            state, &forged, &binding,
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}
