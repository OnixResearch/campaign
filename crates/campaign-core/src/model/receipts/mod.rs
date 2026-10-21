#[cfg(test)]
mod tests;

mod claims;

pub use claims::{NON_CLAIM_COUNT, NonClaim, ReceiptClaims};

const LABEL_RECEIPT: &[u8] = b"decision-receipt-v3";
const LABEL_COST_RESERVATION: &[u8] = b"cost-reservation";
const REFUSAL_ABSENT: u32 = 0;
const REFUSAL_PRESENT: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Bounded structural receipt for one Campaign decision or control event.
// r[impl campaign.receipts]
pub struct CampaignReceipt {
    /// Content-derived receipt identity.
    pub receipt_id: crate::identity::Id,
    /// Campaign identity.
    pub campaign_id: crate::identity::Id,
    /// Current Choregraph projection binding identity the decision was made against.
    pub projection_id: crate::identity::Id,
    /// Frontier state identity.
    pub state_id: crate::identity::Id,
    /// Caller request identity.
    pub request_id: crate::identity::Id,
    /// Selection, stop, wait (derived record), or control-event identity.
    pub decision_id: crate::identity::Id,
    /// Campaign policy identity.
    pub policy_id: crate::identity::Id,
    /// Content-derived selection profile identity.
    pub selection_profile_id: crate::identity::Id,
    /// Declared scheduling policy identity.
    pub scheduling_policy_id: crate::identity::Id,
    /// Declared admission policy identity.
    pub admission_policy_id: crate::identity::Id,
    /// Declared coalescing policy identity.
    pub coalescing_policy_id: crate::identity::Id,
    /// Campaign guidance profile identity.
    pub guidance_profile_id: crate::identity::Id,
    /// Campaign entropy profile identity.
    pub entropy_profile_id: crate::identity::Id,
    /// Campaign budget profile identity.
    pub budget_profile_id: crate::identity::Id,
    /// Guidance record that ranked the selected candidate (selections only).
    pub guidance_record_id: Option<crate::identity::Id>,
    /// Explicit entropy ticket supplied with the request, if any.
    pub entropy_ticket_id: Option<crate::identity::Id>,
    /// Identity of the costs reserved by a selection (selections only).
    pub cost_reservation_id: Option<crate::identity::Id>,
    /// Campaign adapter identity.
    pub adapter_id: crate::identity::Id,
    /// Frontier role projection identity of the state.
    pub role_projection_id: crate::identity::Id,
    /// Admission outcome list of an admission or guidance event (event receipts only).
    pub admission_outcomes_id: Option<crate::identity::Id>,
    /// Rejection reason of a refused candidate batch (refusal receipts only).
    pub admission_refusal: Option<super::candidates::AdmissionRefusalReason>,
    /// Canonical non-claims; always exactly `NonClaim::ALL`.
    pub non_claims: alloc::vec::Vec<NonClaim>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Caller request for the receipt of one candidate batch refused by its declared admission rule.
pub struct RefusalReceiptRequest {
    /// Caller request identity.
    pub request_id: crate::identity::Id,
    /// Refusal produced by `explain_admission_refusal` for the receipt state.
    pub refusal: super::candidates::AdmissionRefusal,
    /// Selection profile of the request; the receipt binds the refusal's own admission and
    /// coalescing policies instead of the profile's.
    pub profile: super::selection::SelectionProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Caller request for a receipt of one accepted admission, guidance, or other control event.
pub struct EventReceiptRequest {
    /// Caller request identity.
    pub request_id: crate::identity::Id,
    /// Accepted control event; its identity must belong to the receipt state. Admission-outcome
    /// identities are derived from this event, never supplied by the caller.
    pub event: super::events::CampaignControlEvent,
    /// Selection profile of the request; candidate-batch receipts bind the event's own admission
    /// and coalescing policies instead of the profile's.
    pub profile: super::selection::SelectionProfile,
}

/// Derives the receipt identity from every bound fact.
///
/// # Errors
///
/// Returns `MissingIdentity` or `CrossedIdentity` for a missing or wrong-kind identity and
/// `InvalidReceiptClaim` when the receipt does not list exactly the canonical non-claims.
// r[impl campaign.receipts]
pub fn derive_receipt_id(
    receipt: &CampaignReceipt,
) -> crate::error::CampaignResult<crate::identity::Id> {
    validate_required_kinds(receipt)?;
    validate_optional_kinds(receipt)?;
    if receipt.non_claims.as_slice() != NonClaim::ALL.as_slice() {
        return Err(crate::error::CampaignError::InvalidReceiptClaim);
    }
    debug_assert_eq!(receipt.non_claims.len(), NON_CLAIM_COUNT);
    debug_assert!(!receipt.decision_id.is_absent());
    crate::identity::derive_id_with(crate::identity::IdKind::Receipt, LABEL_RECEIPT, |writer| {
        let mut writer = writer
            .id(receipt.campaign_id)
            .id(receipt.projection_id)
            .id(receipt.state_id)
            .id(receipt.request_id)
            .id(receipt.decision_id)
            .id(receipt.policy_id)
            .id(receipt.selection_profile_id)
            .id(receipt.scheduling_policy_id)
            .id(receipt.admission_policy_id)
            .id(receipt.coalescing_policy_id)
            .id(receipt.guidance_profile_id)
            .id(receipt.entropy_profile_id)
            .id(receipt.budget_profile_id)
            .option_id(receipt.guidance_record_id)
            .option_id(receipt.entropy_ticket_id)
            .option_id(receipt.cost_reservation_id)
            .id(receipt.adapter_id)
            .id(receipt.role_projection_id)
            .option_id(receipt.admission_outcomes_id);
        writer = match receipt.admission_refusal {
            None => writer.u32(REFUSAL_ABSENT),
            Some(reason) => reason.write(writer.u32(REFUSAL_PRESENT)),
        };
        writer = writer.seq_len(crate::identity::usize_to_u64(receipt.non_claims.len())?);
        for non_claim in &receipt.non_claims {
            writer = writer.u32(claims::non_claim_code(non_claim));
        }
        Ok(writer)
    })
}

fn validate_required_kinds(receipt: &CampaignReceipt) -> crate::error::CampaignResult<()> {
    let required = [
        (receipt.campaign_id, crate::identity::IdKind::Campaign),
        (receipt.projection_id, crate::identity::IdKind::Projection),
        (receipt.state_id, crate::identity::IdKind::State),
        (receipt.request_id, crate::identity::IdKind::Request),
        (receipt.policy_id, crate::identity::IdKind::Policy),
        (
            receipt.selection_profile_id,
            crate::identity::IdKind::SelectionProfile,
        ),
        (
            receipt.scheduling_policy_id,
            crate::identity::IdKind::SchedulingProfile,
        ),
        (
            receipt.admission_policy_id,
            crate::identity::IdKind::AdmissionPolicy,
        ),
        (
            receipt.coalescing_policy_id,
            crate::identity::IdKind::CoalescingPolicy,
        ),
        (
            receipt.guidance_profile_id,
            crate::identity::IdKind::GuidanceProfile,
        ),
        (
            receipt.entropy_profile_id,
            crate::identity::IdKind::EntropyProfile,
        ),
        (
            receipt.budget_profile_id,
            crate::identity::IdKind::BudgetProfile,
        ),
        (receipt.adapter_id, crate::identity::IdKind::Adapter),
        (
            receipt.role_projection_id,
            crate::identity::IdKind::RoleProjection,
        ),
    ];
    for (id, kind) in required {
        id.require_kind(kind)?;
    }
    debug_assert_eq!(receipt.campaign_id.kind, crate::identity::IdKind::Campaign);
    debug_assert_eq!(receipt.state_id.kind, crate::identity::IdKind::State);
    Ok(())
}

fn validate_optional_kinds(receipt: &CampaignReceipt) -> crate::error::CampaignResult<()> {
    let optional = [
        (
            receipt.guidance_record_id,
            crate::identity::IdKind::GuidanceRecord,
        ),
        (
            receipt.entropy_ticket_id,
            crate::identity::IdKind::EntropyTicket,
        ),
        (
            receipt.cost_reservation_id,
            crate::identity::IdKind::CostReservation,
        ),
        (
            receipt.admission_outcomes_id,
            crate::identity::IdKind::AdmissionOutcomes,
        ),
    ];
    for (id, kind) in optional
        .into_iter()
        .filter_map(|(id, kind)| id.map(|id| (id, kind)))
    {
        id.require_kind(kind)?;
    }
    if receipt.decision_id.is_absent() {
        return Err(crate::error::CampaignError::MissingIdentity);
    }
    let is_decision_kind = matches!(
        receipt.decision_id.kind,
        crate::identity::IdKind::Selection
            | crate::identity::IdKind::Stop
            | crate::identity::IdKind::Event
            | crate::identity::IdKind::DerivedRecord
    );
    if !is_decision_kind {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    debug_assert!(!receipt.decision_id.is_absent());
    debug_assert!(is_decision_kind);
    Ok(())
}

/// Derives the identity of the costs one selection reserves.
///
/// # Errors
///
/// Returns a typed `CampaignError` when the selection identity is missing or crossed or a cost
/// list cannot be framed.
pub fn derive_cost_reservation_id(
    selection: &super::publication::SelectionDecision,
) -> crate::error::CampaignResult<crate::identity::Id> {
    selection
        .selection_id
        .require_kind(crate::identity::IdKind::Selection)?;
    crate::identity::derive_id_with(
        crate::identity::IdKind::CostReservation,
        LABEL_COST_RESERVATION,
        |writer| {
            let writer = writer.id(selection.selection_id);
            super::bounds::write_resources(writer, &selection.reserved_costs)
        },
    )
}
