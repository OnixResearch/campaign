//! Retirements and explicit prunes of recipe operations, chosen in suite operation order.

/// The first retained recipe operation in suite order, if any.
pub(super) fn retained(
    session: &crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<Option<crate::vocabulary::OperationHandle>> {
    for operation in super::super::OPERATIONS {
        let Ok(candidate_id) = session.candidate_id(operation) else {
            continue;
        };
        if session.state.candidate(candidate_id)?.status == campaign_core::CandidateStatus::Retained
        {
            return Ok(Some(operation));
        }
    }
    Ok(None)
}

/// Retires `operation` by consumer or by policy.
pub(super) fn retire(
    session: &mut crate::harness::Session<'_, '_>,
    operation: crate::vocabulary::OperationHandle,
    is_policy: bool,
) -> crate::verdicts::ConformanceResult<()> {
    if !is_policy {
        return session.retire(operation);
    }
    let request = campaign_core::RetirementRequest {
        origin: campaign_core::RetirementOrigin::Policy,
        ..session.retirement(operation)?
    };
    let event = campaign_core::retire_candidate(&session.state, request)?;
    session.publish(event)?;
    Ok(())
}

/// Publishes an explicit prune of the `pruned` retained recipe operations without outstanding
/// selections, in suite operation order.
pub(super) fn prune(
    session: &mut crate::harness::Session<'_, '_>,
    pruned: super::super::super::Pruned,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(
        session.state.stopped.is_none(),
        "a recipe prunes before its stop"
    );
    let mut targets = alloc::vec::Vec::with_capacity(super::super::OPERATIONS.len());
    for operation in super::super::OPERATIONS {
        let Ok(candidate_id) = session.candidate_id(operation) else {
            continue;
        };
        let candidate = session.state.candidate(candidate_id)?;
        let is_prunable = candidate.status == campaign_core::CandidateStatus::Retained
            && !session.state.has_outstanding_for_candidate(candidate_id)
            && (pruned != super::super::super::Pruned::Guided
                || candidate.latest_guidance.is_some());
        if is_prunable {
            targets.push((operation, candidate_id));
        }
    }
    if pruned == super::super::super::Pruned::Last {
        targets = targets.split_off(targets.len().saturating_sub(1));
    }
    crate::catalog::common::require(
        !targets.is_empty(),
        "the recipe prunes a retained operation",
    )?;
    debug_assert!(
        pruned != super::super::super::Pruned::Last || targets.len() == 1,
        "a prune of the last candidate names one"
    );
    let request = campaign_core::PruneRequest {
        prune_plan_id: campaign_core::Id::absent(),
        campaign_id: session.state.admission.campaign_id,
        state_id: session.state.state_id,
        candidates: targets
            .iter()
            .map(|(_, candidate_id)| *candidate_id)
            .collect(),
        reason_id: session
            .harness
            .adapter
            .reason(crate::vocabulary::ScenarioReason::Pruning)?,
        retention_profile_id: None,
        rank_order: alloc::vec::Vec::new(),
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    };
    let event = campaign_core::prune_frontier(&session.state, request)?;
    session.publish(event)?;
    session.record(
        crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::PRUNED, "").with_operations(
            targets
                .into_iter()
                .map(|(operation, _)| operation)
                .collect(),
        ),
    );
    Ok(())
}
