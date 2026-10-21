const FIXTURE_LABEL: &[u8] = b"model-receipt-regression";
const OTHER_REQUEST_LABEL: &[u8] = b"other-request";

fn id(kind: crate::identity::IdKind) -> crate::identity::Id {
    crate::identity::derive_id(kind, FIXTURE_LABEL, &[])
}

fn fixture() -> super::CampaignReceipt {
    super::CampaignReceipt {
        receipt_id: crate::identity::Id::absent(),
        campaign_id: id(crate::identity::IdKind::Campaign),
        projection_id: id(crate::identity::IdKind::Projection),
        state_id: id(crate::identity::IdKind::State),
        request_id: id(crate::identity::IdKind::Request),
        decision_id: id(crate::identity::IdKind::Selection),
        policy_id: id(crate::identity::IdKind::Policy),
        selection_profile_id: id(crate::identity::IdKind::SelectionProfile),
        scheduling_policy_id: id(crate::identity::IdKind::SchedulingProfile),
        admission_policy_id: id(crate::identity::IdKind::AdmissionPolicy),
        coalescing_policy_id: id(crate::identity::IdKind::CoalescingPolicy),
        guidance_profile_id: id(crate::identity::IdKind::GuidanceProfile),
        entropy_profile_id: id(crate::identity::IdKind::EntropyProfile),
        budget_profile_id: id(crate::identity::IdKind::BudgetProfile),
        guidance_record_id: Some(id(crate::identity::IdKind::GuidanceRecord)),
        entropy_ticket_id: None,
        cost_reservation_id: Some(id(crate::identity::IdKind::CostReservation)),
        adapter_id: id(crate::identity::IdKind::Adapter),
        role_projection_id: id(crate::identity::IdKind::RoleProjection),
        admission_outcomes_id: None,
        admission_refusal: None,
        non_claims: super::NonClaim::ALL.to_vec(),
    }
}

#[test]
// r[verify campaign.receipts]
fn binds_the_request_without_mutating_input() -> crate::error::CampaignResult<()> {
    let receipt = fixture();
    let before = receipt.clone();
    let original_id = crate::model::derive_receipt_id(&receipt)?;
    assert_eq!(original_id.kind, crate::identity::IdKind::Receipt);
    assert_eq!(receipt, before);
    let mut changed = receipt.clone();
    changed.request_id =
        crate::identity::derive_id(crate::identity::IdKind::Request, OTHER_REQUEST_LABEL, &[]);
    assert_ne!(original_id, crate::model::derive_receipt_id(&changed)?);
    let mut with_outcomes = receipt;
    with_outcomes.admission_outcomes_id = Some(id(crate::identity::IdKind::AdmissionOutcomes));
    assert_ne!(
        original_id,
        crate::model::derive_receipt_id(&with_outcomes)?
    );
    Ok(())
}

#[test]
fn rejects_absent_and_crossed_inputs() {
    let mut receipt = fixture();
    receipt.campaign_id = crate::identity::Id::absent();
    assert_eq!(
        crate::model::derive_receipt_id(&receipt),
        Err(crate::error::CampaignError::MissingIdentity)
    );
    receipt.campaign_id = id(crate::identity::IdKind::Campaign);
    receipt.projection_id = id(crate::identity::IdKind::State);
    assert_eq!(
        crate::model::derive_receipt_id(&receipt),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
    let mut crossed_optional = fixture();
    crossed_optional.cost_reservation_id = Some(id(crate::identity::IdKind::ChildReservation));
    assert_eq!(
        crate::model::derive_receipt_id(&crossed_optional),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
    let mut crossed_decision = fixture();
    crossed_decision.decision_id = id(crate::identity::IdKind::Candidate);
    assert_eq!(
        crate::model::derive_receipt_id(&crossed_decision),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
}

#[test]
fn rejects_a_reordered_or_duplicated_non_claim_list() {
    let mut reordered = fixture();
    reordered.non_claims.swap(0, 1);
    assert_eq!(
        crate::model::derive_receipt_id(&reordered),
        Err(crate::error::CampaignError::InvalidReceiptClaim)
    );
    let mut duplicated = fixture();
    duplicated.non_claims.push(super::NonClaim::SafeDeletion);
    assert_eq!(
        crate::model::derive_receipt_id(&duplicated),
        Err(crate::error::CampaignError::InvalidReceiptClaim)
    );
}
