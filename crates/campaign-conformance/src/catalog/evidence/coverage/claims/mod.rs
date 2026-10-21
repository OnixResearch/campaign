//! Receipts presented at every selected state: the adapter loads the state and is asked for the
//! claims of the event receipt of every control event of its lineage (every sub-kind, for a
//! ranked and an exploratory request), of the plan receipt of every plan the lineage made (each
//! selection and each planner stop before it was recorded, projected when planned), of the
//! state's own ranked and exploratory plan (a recorded stop after it was recorded), and, at a
//! running state, of the reject rule's refusal receipts, under every signal combination.

/// Forbidden effect of a claimed plan receipt of one kind.
const fn plan_name(kind: u32) -> &'static str {
    match kind {
        super::kinds::RANKED_PLAN => {
            "a ranked selection's plan receipt presented at a later state carried a promotion claim"
        }
        super::kinds::EXPLORATORY_PLAN => {
            "an exploratory selection's plan receipt presented at a later state carried a \
             promotion claim"
        }
        _ => "a planner stop's receipt planned before recording carried a promotion claim",
    }
}

/// Presents every receipt kind at the replayed state; returns the kinds presented.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a load, plan, or receipt, and `ForbiddenAccepted`
/// naming the first claimed receipt.
// r[impl campaign.conformance.claims]
// r[impl campaign.receipts.non_claims]
pub(super) fn present(
    session: &mut crate::harness::Session<'_, '_>,
    replayed: &super::recipes::replay::Replayed,
) -> crate::verdicts::ConformanceResult<u32> {
    super::super::claims::witness::present(session)?;
    let mut kinds = session.payloads.iter().fold(0, |kinds, payload| {
        kinds | super::kinds::of_event(&payload.control_event)
    });
    debug_assert!(
        kinds & super::kinds::ADMISSION != 0,
        "every lineage holds its admission"
    );
    debug_assert!(
        kinds & super::kinds::STOP == 0 || session.state.stopped.is_some(),
        "a lineage holding a stop reaches a stopped state"
    );
    let planned: alloc::vec::Vec<(&'static str, campaign_core::CampaignReceipt)> = replayed
        .plans
        .iter()
        .map(|(kind, receipt)| (plan_name(*kind), receipt.clone()))
        .collect();
    super::super::claims::require_unclaimed(session, &planned)?;
    kinds = replayed
        .plans
        .iter()
        .fold(kinds, |kinds, (kind, _)| kinds | kind);
    let ranked = session.plan(None)?;
    let ranked = super::super::claims::plan_receipt(session, &ranked)?;
    let explored = session.plan(Some(super::super::claims::EXPLORE))?;
    let explored = super::super::claims::plan_receipt(session, &explored)?;
    super::super::claims::require_unclaimed(
        session,
        &[
            (
                "a held state's ranked plan receipt carried a promotion claim",
                ranked,
            ),
            (
                "a held state's exploratory plan receipt carried a promotion claim",
                explored,
            ),
        ],
    )?;
    kinds |= super::kinds::STATE_RANKED_PLAN | super::kinds::STATE_EXPLORATORY_PLAN;
    if session.state.stopped.is_none() {
        refusals(session)?;
        kinds |= super::kinds::REFUSAL;
    }
    Ok(kinds)
}

/// Presents the reject rule's refusal receipts at the running state: of an admitted candidate
/// (when the recipe admitted one) and of a duplicate within one batch.
///
/// # Errors
///
/// Returns the adapter or Campaign error of a refusal and `ForbiddenAccepted` naming the first
/// claimed receipt.
fn refusals(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(
        session.state.stopped.is_none(),
        "refusals are presented at running states"
    );
    let rule = session.rule;
    debug_assert!(
        matches!(
            rule.duplicate_candidate,
            campaign_core::DuplicateCandidateRule::Coalesce
        ),
        "the replay coalesces duplicates, so the refusals switch to the reject rule"
    );
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Reject;
    let first = super::recipes::replay::OPERATIONS
        .first()
        .copied()
        .filter(|operation| session.candidate_id(*operation).is_ok());
    let mut receipts = alloc::vec::Vec::with_capacity(2);
    if let Some(admitted) = first {
        receipts.push((
            "a held state's admission refusal receipt carried a promotion claim",
            super::super::claims::duplicates::refusal_receipt(session, &[admitted])?,
        ));
    }
    receipts.push((
        "a held state's same-batch refusal receipt carried a promotion claim",
        super::super::claims::duplicates::refusal_receipt(
            session,
            &[super::super::claims::duplicates::SAME_BATCH_OPERATION; 2],
        )?,
    ));
    session.rule = rule;
    super::super::claims::require_unclaimed(session, &receipts)
}
