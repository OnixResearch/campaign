//! Refusal receipts of the reject rule: an already admitted candidate and a duplicate within one
//! batch.

/// Operation a batch names twice for the reject rule to refuse as a same-batch duplicate.
pub(in crate::catalog::evidence) const SAME_BATCH_OPERATION: crate::vocabulary::OperationHandle =
    crate::vocabulary::OperationHandle(4);

/// Receipts of the reject rule's refusals of an admitted candidate and of a duplicate within one
/// batch.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a refusal, and `Diverged` when a refusal names
/// another duplicate origin.
// r[impl campaign.receipts.non_claims]
pub(super) fn refusal_receipts(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<[campaign_core::CampaignReceipt; 2]> {
    let refusal = refusal_receipt(session, &[super::super::super::common::OP_A])?;
    let same_batch = refusal_receipt(session, &[SAME_BATCH_OPERATION; 2])?;
    let origin = |receipt: &campaign_core::CampaignReceipt| {
        receipt
            .admission_refusal
            .as_ref()
            .map(|refused| refused.duplicate_of)
    };
    super::super::super::common::require(
        origin(&refusal) == Some(campaign_core::DuplicateOrigin::AdmittedCandidate)
            && origin(&same_batch) == Some(campaign_core::DuplicateOrigin::SameBatch),
        "the reject rule refuses an admitted candidate and a duplicate within one batch",
    )?;
    debug_assert_eq!(refusal.state_id, session.state.state_id);
    debug_assert_eq!(same_batch.state_id, session.state.state_id);
    Ok([refusal, same_batch])
}

/// Receipt of the reject rule's refusal of a batch of `operations`.
pub(in crate::catalog::evidence) fn refusal_receipt(
    session: &mut crate::harness::Session<'_, '_>,
    operations: &[crate::vocabulary::OperationHandle],
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    let scenario: alloc::vec::Vec<crate::vocabulary::ScenarioOperation> = operations
        .iter()
        .map(|handle| {
            super::super::super::common::operation(
                *handle,
                crate::harness::history::ROOT,
                super::super::super::common::UNIT_COST,
            )
        })
        .collect();
    let records = session.candidates(&scenario)?;
    let refusal = campaign_core::explain_admission_refusal(
        &session.state,
        records,
        session.rule,
        crate::harness::records::coalescing_policy_id()?,
    )?
    .ok_or(crate::verdicts::ConformanceError::Diverged {
        expected: "the reject rule refuses the duplicate",
    })?;
    super::super::super::common::require(
        refusal.state_id == session.state.state_id
            && refusal.campaign_id == session.state.admission.campaign_id,
        "the refusal binds the current state of the campaign",
    )?;
    debug_assert_eq!(refusal.state_id, session.state.state_id);
    debug_assert_eq!(refusal.campaign_id, session.state.admission.campaign_id);
    let request = campaign_core::RefusalReceiptRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "refusal-receipt")?,
        refusal,
        profile: session.selection_profile(false)?,
    };
    Ok(campaign_core::project_refusal_receipt(
        &session.state,
        &request,
        &session.binding()?,
    )?)
}
