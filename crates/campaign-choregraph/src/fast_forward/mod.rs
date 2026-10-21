//! Stateful fast-forward: advancing a previously validated control projection.

/// Validates a caller-selected control projection as a fast-forward of `previous`.
///
/// The new lineage must extend the previously validated one: the previous head is the new
/// lineage's event at the previous length - 1, and every previous control event is an equal
/// prefix of the supplied payloads. Generation arithmetic alone cannot tell an extension from a
/// branch moved onto a sibling fork of the same depth, so a host that holds a validated projection
/// advances through this function. Only the new suffix is validated as history content and
/// recorded publication fences, and applied incrementally to the previous state; the result equals
/// the cold-start validation of the same projection.
///
/// # Errors
///
/// Returns `ProjectionBindingMismatch` if the request does not bind its graph and branch,
/// `Campaign(CrossedIdentity)` for projection schema, adapter, or control-domain identities other
/// than the admission's (and so `previous`'s), `ControlBranchMismatch` for a branch other than
/// `previous`'s, `ProjectionPayloadMismatch` if the payloads are not the lineage,
/// `NonFastForwardBranch` if the lineage does not extend `previous` (a rewound branch or a fork),
/// `PayloadIdentityMismatch` if a payload rewrites a previously accepted event,
/// `PublicationFenceMismatch` for a new selection planned against another branch value, the
/// history, envelope, or Campaign error of a new event, `StaleGraph` for a graph lacking events
/// `previous` recorded (fewer events than its graph, or a Campaign control draft it recorded as
/// pending: a fast-forward never goes back in graph time), and `SharedControlGraph` for a graph
/// holding another Campaign control root under the envelope.
///
/// # Panics
///
/// Panics if accepted projection binding or payload-count invariants fail.
// r[impl campaign.history_projection.stale]
// r[impl campaign.history_projection.older_graph]
pub fn validate_control_advance(
    previous: &crate::contracts::ValidatedControlProjection,
    request: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<crate::contracts::ValidatedControlProjection> {
    let binding = crate::lineage::bind(request)?;
    let projection = crate::lineage::project(request)?;
    let events = projection.events();
    let start = extension_start(previous, &binding, events)?;
    let admitted = &previous.state.admission.projection;
    crate::lineage::check_fast_forward(&binding, admitted, events.len())?;
    check_accepted_payloads(previous, events, request.payloads)?;
    let suffix = crate::lineage::validate_payloads(request, events, start)?;
    crate::lineage::check_recorded_fences(
        request.branch,
        admitted.generation,
        events,
        &suffix,
        start,
    )?;
    let mut state = previous.state.clone();
    for event in &suffix {
        state = campaign_core::apply_control_event(state, event)?;
    }
    let mut control_events = alloc::vec::Vec::with_capacity(events.len());
    control_events.extend_from_slice(&previous.control_events);
    control_events.extend(suffix);
    assert_eq!(binding, request.expected_binding);
    assert_eq!(control_events.len(), events.len());
    debug_assert_eq!(state.admission, previous.state.admission);
    if request.graph.len() < previous.graph_events
        || !previous
            .pending
            .drafts
            .iter()
            .all(|draft| request.graph.contains(*draft))
    {
        return Err(crate::contracts::AdapterError::StaleGraph);
    }
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

/// Requires the advanced binding to name the previous control branch and the admission's
/// projection identities,
/// and the previous head to be the lineage event at the previous length - 1. Returns the lineage
/// index of the first new event.
///
/// History events are content-addressed over their parents, so the lineage below that event is
/// exactly the previously validated lineage.
fn extension_start(
    previous: &crate::contracts::ValidatedControlProjection,
    current: &campaign_core::ProjectionBinding,
    events: &[choregraph_history::EventId],
) -> crate::contracts::AdapterResult<usize> {
    if current.control_branch_name_id != previous.binding.control_branch_name_id {
        return Err(crate::contracts::AdapterError::ControlBranchMismatch);
    }
    let admitted = &previous.state.admission.projection;
    if current.projection_schema_id != admitted.projection_schema_id
        || current.adapter_id != admitted.adapter_id
        || current.control_domain_id != admitted.control_domain_id
    {
        return Err(campaign_core::CampaignError::CrossedIdentity.into());
    }
    debug_assert_eq!(
        previous.binding.adapter_id,
        previous.state.admission.projection.adapter_id
    );
    let start = previous.control_events.len();
    let head_index = start
        .checked_sub(1)
        .ok_or(campaign_core::CampaignError::MissingAdmission)?;
    let is_extension = events.get(head_index).is_some_and(|event| {
        crate::identity::head_id_from_event(*event) == previous.binding.head_id
    });
    if !is_extension {
        return Err(crate::contracts::AdapterError::NonFastForwardBranch);
    }
    debug_assert!(events.len() >= start);
    Ok(start)
}

/// Requires the payloads of the previously accepted events to be exactly those events.
fn check_accepted_payloads(
    previous: &crate::contracts::ValidatedControlProjection,
    events: &[choregraph_history::EventId],
    payloads: &[crate::contracts::ControlPayloadEnvelope],
) -> crate::contracts::AdapterResult<()> {
    for ((history_event_id, payload), accepted) in
        events.iter().zip(payloads).zip(&previous.control_events)
    {
        if payload.history_event_id != *history_event_id {
            return Err(crate::contracts::AdapterError::ProjectionPayloadMismatch);
        }
        if payload.control_event != *accepted {
            return Err(crate::contracts::AdapterError::PayloadIdentityMismatch);
        }
    }
    debug_assert!(payloads.len() >= previous.control_events.len());
    Ok(())
}
