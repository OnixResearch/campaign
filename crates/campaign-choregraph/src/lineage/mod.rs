//! Linear control-projection validation and Campaign decision planning.

/// Lineage index of the Campaign admission, the root of every control lineage.
const ADMISSION_INDEX: usize = 0;

/// Validates a caller-selected linear control projection from its root and reconstructs state.
///
/// This is the cold-start load: the branch must be the admission's named control branch and must
/// have advanced exactly one generation per control event since the admission's binding, and every
/// recorded selection's publication fence must be the branch value at its lineage position. A
/// host that already holds a validated projection advances through
/// [`crate::validate_control_advance`] instead, which also rejects a branch moved onto a fork.
///
/// # Errors
///
/// Returns an error if the graph, projection, envelope, payload, or core event is not exact,
/// `Campaign(CrossedIdentity)` for a binding under another projection schema, adapter, or control
/// domain than the admission's, `ControlBranchMismatch` for a branch other than the admission's,
/// `NonFastForwardBranch` for a rewound branch value, `PublicationFenceMismatch` for a recorded
/// selection planned against another branch value, and `SharedControlGraph` for a graph holding
/// another Campaign control root under the envelope (one Campaign campaign per graph and
/// envelope).
///
/// # Panics
///
/// Panics if accepted projection binding or payload-count invariants fail.
// r[impl campaign.history_projection]
pub fn validate_control_projection(
    request: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<crate::contracts::ValidatedControlProjection> {
    let binding = bind(request)?;
    let projection = project(request)?;
    let events = projection.events();
    let control_events = validate_payloads(request, events, ADMISSION_INDEX)?;
    let state = campaign_core::reconstruct(&control_events)?;
    check_fast_forward(&binding, &state.admission.projection, control_events.len())?;
    check_recorded_fences(
        request.branch,
        state.admission.projection.generation,
        events,
        &control_events,
        ADMISSION_INDEX,
    )?;
    assert_eq!(binding, request.expected_binding);
    assert_eq!(control_events.len(), events.len());
    let root = *events
        .first()
        .ok_or(campaign_core::CampaignError::MissingAdmission)?;
    let pending = crate::occupancy::scan(request.graph, &binding, root, request.branch.head)?;
    let reserve = crate::headroom::reserve::record(
        crate::headroom::limits::Extent {
            lineage: control_events.len(),
            graph: request.graph.len(),
        },
        &state,
        pending.drafts.len(),
    )?;
    Ok(crate::contracts::ValidatedControlProjection {
        binding,
        choregraph_projection_identity: projection.identity(),
        control_events,
        state,
        maximum_events: request.maximum_events,
        graph_events: request.graph.len(),
        graph_max_events: request.graph.limits().max_events,
        pending,
        reserve,
    })
}

/// Derives the binding of the request's graph and branch and requires the caller's expected one.
// r[impl campaign.history_projection.stale]
pub(crate) fn bind(
    request: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<campaign_core::ProjectionBinding> {
    request.expected_binding.validate()?;
    let binding = crate::identity::derive_projection_binding(
        request.graph,
        request.branch,
        request.expected_binding.projection_schema_id,
        request.expected_binding.adapter_id,
        request.expected_binding.control_domain_id,
    )?;
    if binding != request.expected_binding {
        return Err(crate::contracts::AdapterError::ProjectionBindingMismatch);
    }
    Ok(binding)
}

/// Selects the branch head's linear lineage and requires one payload per lineage event.
pub(crate) fn project(
    request: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<choregraph_history::Projection> {
    let projection = choregraph_history::linear_lineage(
        request.graph,
        request.branch.head,
        request.parent_choices,
        request.maximum_events,
    )?;
    if projection.events().len() != request.payloads.len() {
        return Err(crate::contracts::AdapterError::ProjectionPayloadMismatch);
    }
    Ok(projection)
}

/// Requires the branch to be the admission's named control branch under the admission's projection
/// schema, adapter, and control domain, advanced exactly one generation per control event since
/// the admission's binding, so a rewound or foreign branch value never validates as current
/// history.
// r[impl campaign.history_projection.stale]
pub(crate) fn check_fast_forward(
    current: &campaign_core::ProjectionBinding,
    admitted: &campaign_core::ProjectionBinding,
    lineage_len: usize,
) -> crate::contracts::AdapterResult<()> {
    if current.projection_schema_id != admitted.projection_schema_id
        || current.adapter_id != admitted.adapter_id
        || current.control_domain_id != admitted.control_domain_id
    {
        return Err(campaign_core::CampaignError::CrossedIdentity.into());
    }
    if current.control_branch_name_id != admitted.control_branch_name_id {
        return Err(crate::contracts::AdapterError::ControlBranchMismatch);
    }
    let lineage =
        u64::try_from(lineage_len).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let expected_generation = admitted
        .generation
        .checked_add(lineage)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    if current.generation != expected_generation {
        return Err(crate::contracts::AdapterError::NonFastForwardBranch);
    }
    debug_assert!(current.generation > admitted.generation);
    Ok(())
}

/// Validates the payloads of the lineage events from `start` on, each the linear child of the
/// event before it, and returns their control events.
pub(crate) fn validate_payloads(
    request: &crate::contracts::ControlProjectionRequest<'_>,
    events: &[choregraph_history::EventId],
    start: usize,
) -> crate::contracts::AdapterResult<alloc::vec::Vec<campaign_core::CampaignControlEvent>> {
    let (Some(suffix), Some(payloads)) = (events.get(start..), request.payloads.get(start..))
    else {
        return Err(crate::contracts::AdapterError::ProjectionPayloadMismatch);
    };
    let mut control_events = alloc::vec::Vec::with_capacity(payloads.len());
    let mut parent = start
        .checked_sub(1)
        .and_then(|index| events.get(index).copied());
    for (history_event_id, payload) in suffix.iter().copied().zip(payloads) {
        if payload.history_event_id != history_event_id {
            return Err(crate::contracts::AdapterError::ProjectionPayloadMismatch);
        }
        crate::envelope::validate_history_content(
            request.graph,
            history_event_id,
            &payload.control_event,
            &request.expected_binding,
            parent,
        )?;
        control_events.push(payload.control_event.clone());
        parent = Some(history_event_id);
    }
    assert_eq!(control_events.len(), suffix.len());
    assert_eq!(parent, events.last().copied());
    Ok(control_events)
}

/// Requires every recorded selection's publication fence to be the branch value it was planned
/// against: the named control branch at the preceding lineage event, one generation per earlier
/// control event past the admission's binding.
///
/// `control_events` are the control events of the lineage events from `start` on.
pub(crate) fn check_recorded_fences(
    branch: &choregraph_history::BranchRef,
    admitted_generation: u64,
    events: &[choregraph_history::EventId],
    control_events: &[campaign_core::CampaignControlEvent],
    start: usize,
) -> crate::contracts::AdapterResult<()> {
    let mut fence = branch.clone();
    for (offset, control_event) in control_events.iter().enumerate() {
        if let campaign_core::CampaignControlEvent::SelectionPublished(event) = control_event {
            let index = start
                .checked_add(offset)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
            let parent = index
                .checked_sub(1)
                .and_then(|parent| events.get(parent).copied())
                .ok_or(crate::contracts::AdapterError::PublicationFenceMismatch)?;
            let lineage = u64::try_from(index)
                .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
            fence.head = parent;
            fence.generation = admitted_generation
                .checked_add(lineage)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
            if plan_fact(&event.decision.publication_plan) != crate::identity::branch_fact(&fence) {
                return Err(crate::contracts::AdapterError::PublicationFenceMismatch);
            }
        }
    }
    debug_assert_eq!(fence.name, branch.name);
    Ok(())
}

/// The branch value a selection's publication plan was planned against.
const fn plan_fact(plan: &campaign_core::PublicationPlan) -> campaign_core::BranchFact {
    campaign_core::BranchFact {
        control_branch_id: plan.control_branch_id,
        head_id: plan.expected_head_id,
        generation: plan.expected_generation,
    }
}

/// Plans the next Campaign decision after exact published projection validation, within the host
/// control capacity publication enforces.
///
/// While the graph holds a Campaign control draft pending on the branch head, nothing is planned:
/// the host completes that publication first (see [`crate::pending_control_draft`]). Without room
/// in the host capacity for another selection, its settlement, and the stop (and, for a wait for
/// guidance, the guidance event before it), a plan towards a new selection becomes
/// `Wait(AwaitingResults)` while selections are outstanding and otherwise the consumer stop under
/// [`crate::control_capacity_reason_id`]. The capacity starts from the demand the projection's
/// reserve records ([`crate::ValidatedControlProjection::reserved_control_events`] and
/// [`crate::ValidatedControlProjection::reserved_graph_events`]), the accounting publication
/// enforces.
///
/// # Errors
///
/// Returns an error if the request does not bind the current Choregraph generation and head,
/// `PendingControlDraft` while a control draft is pending on the head, the Campaign planner's
/// typed error, and a capacity error when the planned stop no longer fits.
// r[impl campaign.history_projection.caller_binding]
// r[impl campaign.history_projection.stale]
// r[impl campaign.publication_fence.pending]
pub fn plan_next_from_published_projection(
    current: &crate::contracts::ValidatedControlProjection,
    request: crate::contracts::PublishedSelectionRequest,
) -> crate::contracts::AdapterResult<campaign_core::DecisionPlan> {
    if request.request.expected_generation != current.binding.generation
        || request.request.expected_head_id != current.binding.head_id
    {
        return Err(crate::contracts::AdapterError::ProjectionBindingMismatch);
    }
    crate::exclusivity::check_none(current.pending.first)?;
    let plan = campaign_core::plan_next(&current.state, request.request, &current.binding)?;
    crate::headroom::plan_within_capacity(current, plan)
}
