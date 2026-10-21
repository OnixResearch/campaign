//! Immutable control-event publication planning and execution admission.

pub(crate) mod genesis;
#[cfg(test)]
mod tests;

/// The identity `draft` is admitted under, and whether `graph` already holds exactly that draft.
pub(crate) fn held_draft(
    graph: &choregraph_history::HistoryGraph,
    draft: &choregraph_history::EventDraft,
) -> (choregraph_history::EventId, bool) {
    let identity = choregraph_history::HistoryGraph::expected_identity(draft);
    let is_held = graph
        .event(identity)
        .is_some_and(|event| event.content() == draft);
    (identity, is_held)
}

/// Plans one immutable control-event publication and one branch advance.
///
/// The caller's current projection inputs are re-validated as a fast-forward of the projection
/// the event was computed from (`request.previous`), the event's planned fence must name the
/// validated branch value, and the event must apply to the reconstructed Campaign state, so a
/// draft is only admitted for an event that the control lineage will replay.
///
/// Republication is idempotent: when the current graph already holds exactly the event's draft
/// (a host that durably admitted the draft but not its branch compare-and-set), the draft is not
/// admitted again and the publication advances the branch to the existing event. It carries the
/// original publication's payload, draft, event, graph, branch plan, and control event, bound to
/// the current projection (whose graph identity now includes the draft). Any other event is
/// refused while a control draft is pending on the branch head, so a stale publisher or a
/// compare-and-set loser never admits a draft that consumes the winning lineage's reserve.
///
/// Hosts publish only on durably committed branch values: they never plan or publish through a
/// publication's planned branch value before its compare-and-set is durable. A pipelining host
/// that does leaves drafts chained below the pending draft, completed in chain order where they
/// still fit: a draft chained below a pending draft was planned on another branch value, so
/// before the draft above it commits it is not a republication and is refused
/// (`PublicationFenceMismatch` for a selection, `Campaign(StaleState)` for every other event
/// built by publication, whose state before is the state after the draft above it).
///
/// Every validated projection's reserve counts each Campaign control draft pending below the
/// branch head as a selection (its event and one settlement; see
/// [`crate::ValidatedControlProjection::reserved_graph_events`]). A draft is only admitted when
/// its actual demand (the lineage and graph after it, its outstanding settlements, and the stop)
/// fits, and its republication checks that same demand plus the drafts the graph holds chained
/// below it (each counted as a selection). Without chained drafts that never exceeds the counted
/// reserve, so a host that publishes only on durably committed branch values and keeps its other
/// appends within the exposed [`crate::ValidatedControlProjection::graph_room`] can always
/// republish its own pending draft, settle every outstanding selection, and publish the stop (of
/// racing siblings whose joint demand exceeds the capacity, only a draft whose actual demand still
/// fits republishes). With chained drafts the count is conservative (one event over for a chained
/// draft of other new work, two for a chained settlement or stop), so a pipelined chain can be
/// refused at a bound and block the head: the pipelined-chain residual (see
/// [`crate::ValidatedControlProjection::graph_room`]).
///
/// # Errors
///
/// Returns an error if the current graph, branch, binding, lineage, or payloads are stale or
/// malformed, every error of [`crate::validate_control_advance`] (`NonFastForwardBranch` if they
/// do not extend the previous projection, `StaleGraph` for a graph older than the previous
/// projection's, `SharedControlGraph` for a graph holding another campaign's control root), if
/// the event's fence is stale, the Campaign error that replay would raise for the event on the
/// current state (`StaleState` for the event of a non-selection draft chained below the pending
/// draft), `PendingControlDraft` when the graph holds another control draft whose parent
/// is the branch head, and `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or
/// `GraphCapacityExhausted` when the lineage after the event, its outstanding settlements, the
/// stop reserve of an open campaign, and (for a republication) the drafts chained below it, each
/// counted as a selection, no longer fit the projection bound (`request.current.maximum_events`),
/// the branch generation range, or the graph's event limit
/// (`request.current.graph.limits().max_events`, counting every event the graph holds); for a
/// pipelined chain this can refuse a republication no other writer's append made unfit.
///
/// # Panics
///
/// Panics if the pinned branch planner changes the prior branch or loses the admitted head.
// r[impl campaign.publication_fence]
// r[impl campaign.publication_fence.republication]
// r[impl campaign.publication_fence.pending]
// r[impl campaign.publication_fence.pending_several]
// r[impl campaign.publication_fence.durable_base]
pub fn plan_control_event_publication(
    request: crate::contracts::ControlPublicationRequest<'_>,
) -> crate::contracts::AdapterResult<crate::contracts::ControlPublication> {
    let current =
        crate::fast_forward::validate_control_advance(request.previous, &request.current)?;
    crate::admission::check_selection_fence(&request.control_event, &current.binding)?;
    let state_after = campaign_core::apply_control_event(current.state, &request.control_event)?;
    debug_assert!(
        state_after
            .event_ids
            .contains(&request.control_event.event_id())
    );
    let branch = request.current.branch;
    let binding = current.binding;
    let payload_id = crate::identity::derive_control_payload_id(&request.control_event)?;
    let draft = crate::envelope::control_event_draft(
        &request.control_event,
        binding.projection_schema_id,
        binding.adapter_id,
        binding.control_domain_id,
        Some(branch.head),
    )?;
    let graph = request.current.graph;
    let (existing, is_republication) = held_draft(graph, &draft);
    let chained = if is_republication {
        crate::occupancy::chained_below(graph, &binding, existing)
    } else {
        crate::exclusivity::check_none(current.pending.first)?;
        0
    };
    let after = crate::headroom::limits::Extent {
        lineage: current
            .control_events
            .len()
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        graph: graph
            .len()
            .checked_add(u32::from(!is_republication))
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
    };
    crate::headroom::check_publication(&state_after, after, chained, &request.current)?;
    let (history_event_id, graph_after_admission) = if is_republication {
        (existing, graph.clone())
    } else {
        let admission = graph.admit(draft.clone())?;
        (admission.event, admission.graph)
    };
    debug_assert_eq!(graph_after_admission.len(), after.graph);
    let branch_plan = choregraph_history::plan_branch_advance(
        &graph_after_admission,
        branch,
        binding.generation,
        branch.head,
        history_event_id,
    )?;
    assert_eq!(branch_plan.before, *branch);
    assert_eq!(branch_plan.after.head, history_event_id);
    Ok(crate::contracts::ControlPublication {
        payload_id,
        draft,
        history_event_id,
        graph_after_admission,
        branch_plan,
        control_event: request.control_event,
        binding,
    })
}

/// Returns true only after exact durable publication observations are supplied.
///
/// The selection must be exactly the published event's decision, and the publication's draft,
/// event identity, and branch plan must encode that event. The Campaign fence evidence is built
/// from the actual published control event and the publication's own branch plan, never from the
/// selection's planned fence.
///
/// # Errors
///
/// Returns `MissingControlEvent` if the publication does not publish exactly this selection, an
/// envelope error if the publication does not encode its control event, `PublicationEvidenceMismatch`
/// if the evidence does not match the planned event and branch update, and the Campaign error if
/// the selection does not re-derive or the publication fence rejects the branch transition.
// r[impl campaign.publication_fence]
pub fn can_execute_selection_after_publication(
    selection: &campaign_core::SelectionDecision,
    publication: &crate::contracts::ControlPublication,
    evidence: &crate::contracts::DurablePublicationEvidence,
) -> crate::contracts::AdapterResult<bool> {
    let campaign_core::CampaignControlEvent::SelectionPublished(event) = &publication.control_event
    else {
        return Err(crate::contracts::AdapterError::MissingControlEvent);
    };
    if event.decision != *selection {
        return Err(crate::contracts::AdapterError::MissingControlEvent);
    }
    crate::admission::check_publication_encoding(publication)?;
    if evidence.history_event_id != publication.history_event_id
        || evidence.branch_after.head != publication.history_event_id
    {
        return Err(crate::contracts::AdapterError::PublicationEvidenceMismatch);
    }
    if evidence.branch_before != publication.branch_plan.before
        || evidence.branch_after != publication.branch_plan.after
    {
        return Err(crate::contracts::AdapterError::PublicationEvidenceMismatch);
    }
    let core_evidence = campaign_core::PublicationEvidence {
        event_id: event.event_id,
        selection_id: event.decision.selection_id,
        branch_before: crate::identity::branch_fact(&publication.branch_plan.before),
        branch_after: crate::identity::branch_fact(&publication.branch_plan.after),
        durable_event: evidence.durable_event,
        durable_branch_update: evidence.durable_branch_update,
    };
    debug_assert_eq!(evidence.branch_after.head, publication.history_event_id);
    Ok(campaign_core::can_execute_published_selection(
        selection,
        &core_evidence,
    )?)
}

/// Rejects a publication whose payload identity, draft, event identity, admitted content, or
/// branch plan does not encode its control event on its bound branch value.
fn check_publication_encoding(
    publication: &crate::contracts::ControlPublication,
) -> crate::contracts::AdapterResult<()> {
    let payload_id = crate::identity::derive_control_payload_id(&publication.control_event)?;
    if publication.payload_id != payload_id
        || publication.draft.payload_identity != crate::identity::digest_from_id(payload_id)
    {
        return Err(crate::contracts::AdapterError::PayloadIdentityMismatch);
    }
    let before = &publication.branch_plan.before;
    let expected = crate::envelope::control_event_draft(
        &publication.control_event,
        publication.binding.projection_schema_id,
        publication.binding.adapter_id,
        publication.binding.control_domain_id,
        Some(before.head),
    )?;
    if publication.draft.metadata != expected.metadata {
        return Err(crate::contracts::AdapterError::MetadataMismatch);
    }
    if publication.draft != expected
        || choregraph_history::HistoryGraph::expected_identity(&publication.draft)
            != publication.history_event_id
        || publication.branch_plan.after.head != publication.history_event_id
    {
        return Err(crate::contracts::AdapterError::PublicationEvidenceMismatch);
    }
    let admitted = publication
        .graph_after_admission
        .event(publication.history_event_id)
        .ok_or(crate::contracts::AdapterError::PublicationEvidenceMismatch)?;
    if *admitted.content() != publication.draft {
        return Err(crate::contracts::AdapterError::PublicationEvidenceMismatch);
    }
    if crate::identity::branch_fact(before) != binding_fact(&publication.binding) {
        return Err(crate::contracts::AdapterError::PublicationFenceMismatch);
    }
    debug_assert_eq!(publication.draft, expected);
    debug_assert_eq!(
        publication.branch_plan.after.head,
        publication.history_event_id
    );
    Ok(())
}

const fn binding_fact(binding: &campaign_core::ProjectionBinding) -> campaign_core::BranchFact {
    campaign_core::BranchFact {
        control_branch_id: binding.control_branch_id,
        head_id: binding.head_id,
        generation: binding.generation,
    }
}

/// Rejects a selection whose planned publication fence is not the current branch value.
// r[impl campaign.publication_fence.stale]
pub(super) fn check_selection_fence(
    control_event: &campaign_core::CampaignControlEvent,
    current: &campaign_core::ProjectionBinding,
) -> crate::contracts::AdapterResult<()> {
    let campaign_core::CampaignControlEvent::SelectionPublished(event) = control_event else {
        return Ok(());
    };
    let plan = &event.decision.publication_plan;
    if plan.control_branch_id != current.control_branch_id
        || plan.expected_head_id != current.head_id
        || plan.expected_generation != current.generation
    {
        return Err(crate::contracts::AdapterError::PublicationFenceMismatch);
    }
    Ok(())
}
