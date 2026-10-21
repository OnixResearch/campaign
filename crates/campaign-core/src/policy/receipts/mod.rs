mod recorded;

pub use recorded::{project_event_receipt, project_refusal_receipt};

const LABEL_WAIT_DECISION: &[u8] = b"wait-decision";
const WAIT_GUIDANCE: u32 = 1;
const WAIT_RESULTS: u32 = 2;
const WAIT_CAPACITY: u32 = 3;

/// Projects the receipt of one planned decision made against `current_projection`.
///
/// The decision must be exactly the one `plan_next` makes for `request` on `state` against
/// `current_projection`: the projection re-plans and binds only a genuine decision.
///
/// # Errors
///
/// Returns `StaleState` when the state is not current (canonical order and re-derived identity),
/// `MissingAdmission` for a state without its admission event (zero control events),
/// the binding's typed error when it is malformed, `CrossedIdentity` when the binding or request
/// belongs to another campaign, state, or history (another projection schema, adapter, control
/// domain or control-branch name, or a generation other than the admission generation plus the
/// state's control-event count) or when `plan` differs from the re-planned decision,
/// `ArithmeticOverflow` when that generation overflows, and the planner's typed error (for
/// example `InvalidEntropy`) for a request it rejects.
// r[impl campaign.receipts]
// r[impl campaign.receipts.host_capacity]
// r[impl campaign.receipts.crossed]
pub fn project_receipt(
    state: &crate::frontier::FrontierState,
    plan: &crate::model::DecisionPlan,
    request: &crate::model::SelectionRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    crate::policy::validation::assert_state_identity_current(state)?;
    require_state_history(state, current_projection)?;
    require_request_of_state(state, request, current_projection)?;
    let planned = crate::policy::decision::replan(
        state,
        &crate::policy::decision::PlanningInputs::of_request(request, current_projection),
    )?;
    if planned != *plan {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let mut receipt = base_receipt(
        state,
        current_projection,
        request.request_id,
        &request.profile,
        crate::identity::Id::absent(),
    )?;
    receipt.entropy_ticket_id = request.entropy.as_ref().map(|ticket| ticket.entropy_id);
    match &plan.decision {
        crate::model::NextDecision::Select(selection) => {
            receipt.decision_id = selection.selection_id;
            receipt.guidance_record_id = Some(selection.guidance_id);
            receipt.cost_reservation_id =
                Some(crate::model::derive_cost_reservation_id(selection)?);
            debug_assert_eq!(receipt.entropy_ticket_id, selection.entropy_id);
        }
        crate::model::NextDecision::Stop(stop) => receipt.decision_id = stop.stop_id,
        crate::model::NextDecision::Wait(reason) => {
            receipt.decision_id = derive_wait_decision_id(state, request.request_id, reason)?;
        }
    }
    debug_assert_eq!(receipt.state_id, plan.state_id);
    debug_assert_eq!(receipt.request_id, request.request_id);
    finish(receipt)
}

/// Validates a projected receipt and the claims a consumer attaches to it.
///
/// # Errors
///
/// Returns `InvalidReceiptClaim` when the receipt omits a non-claim or any claim is asserted and
/// `MalformedIdentity` when the receipt identity does not match its content.
pub fn validate_receipt(
    receipt: &crate::model::CampaignReceipt,
    claims: &crate::model::ReceiptClaims,
) -> crate::error::CampaignResult<()> {
    claims.validate_no_claims()?;
    if crate::model::derive_receipt_id(receipt)? != receipt.receipt_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    Ok(())
}

/// Requires `current_projection` to be the state's own current history value: a re-derivable
/// binding of the admitted projection schema, adapter, control domain and control-branch name, at
/// exactly the admission generation plus one generation per control event (the adapter's
/// fast-forward rule). Callers check state currency first, which re-derives the admission and its
/// admitted binding, so the branch name cannot be edited under an unchanged campaign identity. A
/// state without its admission event (zero control events, only caller-built) has no history:
/// `MissingAdmission`.
// r[impl campaign.history_projection.caller_binding]
// r[impl campaign.receipts.crossed]
pub(super) fn require_state_history(
    state: &crate::frontier::FrontierState,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<()> {
    if state.control_event_count == 0 {
        return Err(crate::error::CampaignError::MissingAdmission);
    }
    current_projection.validate()?;
    let admitted = &state.admission.projection;
    debug_assert!(admitted.validate().is_ok());
    let is_admitted_envelope = current_projection.projection_schema_id
        == admitted.projection_schema_id
        && current_projection.adapter_id == state.admission.adapter_id
        && current_projection.control_domain_id == state.admission.control_domain_id;
    let generation = admitted
        .generation
        .checked_add(state.control_event_count)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    let is_admitted_branch_value = current_projection.control_branch_name_id
        == admitted.control_branch_name_id
        && current_projection.generation == generation;
    if !(is_admitted_envelope && is_admitted_branch_value) {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    debug_assert!(current_projection.generation > admitted.generation);
    Ok(())
}

fn require_request_of_state(
    state: &crate::frontier::FrontierState,
    request: &crate::model::SelectionRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<()> {
    request
        .request_id
        .require_kind(crate::identity::IdKind::Request)?;
    let is_request_of_state =
        request.campaign_id == state.admission.campaign_id && request.state_id == state.state_id;
    let is_request_of_history = request.expected_generation == current_projection.generation
        && request.expected_head_id == current_projection.head_id;
    if !(is_request_of_state && is_request_of_history) {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    Ok(())
}

pub(super) fn base_receipt(
    state: &crate::frontier::FrontierState,
    current_projection: &crate::model::ProjectionBinding,
    request_id: crate::identity::Id,
    profile: &crate::model::SelectionProfile,
    decision_id: crate::identity::Id,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    let roles = crate::discipline::project_frontier_roles(state)?;
    debug_assert_eq!(current_projection.adapter_id, state.admission.adapter_id);
    debug_assert_eq!(
        roles.role_projection_id.kind,
        crate::identity::IdKind::RoleProjection
    );
    Ok(crate::model::CampaignReceipt {
        receipt_id: crate::identity::Id::absent(),
        campaign_id: state.admission.campaign_id,
        projection_id: current_projection.projection_id,
        state_id: state.state_id,
        request_id,
        decision_id,
        policy_id: state.admission.policy_id,
        selection_profile_id: profile.profile_id,
        scheduling_policy_id: profile.scheduling_policy_id,
        admission_policy_id: profile.admission_policy_id,
        coalescing_policy_id: profile.coalescing_policy_id,
        guidance_profile_id: state.admission.guidance_profile_id,
        entropy_profile_id: state.admission.entropy_profile_id,
        budget_profile_id: state.admission.budget_profile_id,
        guidance_record_id: None,
        entropy_ticket_id: None,
        cost_reservation_id: None,
        adapter_id: state.admission.adapter_id,
        role_projection_id: roles.role_projection_id,
        admission_outcomes_id: None,
        admission_refusal: None,
        non_claims: crate::model::NonClaim::ALL.to_vec(),
    })
}

pub(super) fn finish(
    mut receipt: crate::model::CampaignReceipt,
) -> crate::error::CampaignResult<crate::model::CampaignReceipt> {
    receipt.receipt_id = crate::model::derive_receipt_id(&receipt)?;
    debug_assert_eq!(receipt.receipt_id.kind, crate::identity::IdKind::Receipt);
    debug_assert_eq!(
        receipt.non_claims.as_slice(),
        crate::model::NonClaim::ALL.as_slice()
    );
    Ok(receipt)
}

fn derive_wait_decision_id(
    state: &crate::frontier::FrontierState,
    request_id: crate::identity::Id,
    reason: &crate::model::WaitReason,
) -> crate::error::CampaignResult<crate::identity::Id> {
    let code = match reason {
        crate::model::WaitReason::AwaitingGuidance => WAIT_GUIDANCE,
        crate::model::WaitReason::AwaitingResults => WAIT_RESULTS,
        crate::model::WaitReason::AwaitingCapacity => WAIT_CAPACITY,
    };
    let wait_id = crate::identity::derive_id_with(
        crate::identity::IdKind::DerivedRecord,
        LABEL_WAIT_DECISION,
        |writer| {
            Ok(writer
                .id(state.admission.campaign_id)
                .id(state.state_id)
                .id(request_id)
                .u32(code))
        },
    )?;
    debug_assert_eq!(wait_id.kind, crate::identity::IdKind::DerivedRecord);
    Ok(wait_id)
}
