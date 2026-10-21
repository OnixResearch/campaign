//! Admission of a control root: the first event of a control lineage, without a branch plan.

/// Admits one root Campaign control event without a branch mutation plan.
///
/// The event must be a `CampaignAdmitted` event that reconstructs a Campaign state, and the
/// admission's projection schema, adapter, and control domain must be the supplied identities, so
/// a campaign's control root is only ever drafted under the identities its admission binds. The
/// graph must hold no other Campaign control event under those identities: one Campaign campaign
/// per graph and envelope, since the adapter bounds one campaign's reserve against the graph's
/// event limit (a host gives each campaign its own graph). The root must leave room for its own
/// stop: two control events within the graph's projection limit (`max_projection_events`), the
/// admission generation plus two without overflow, and the graph holding the root and the stop
/// within its event limit (`graph.len() + 2 <= max_events`).
///
/// Admission is idempotent: when the graph already holds exactly the root's draft (a host that
/// durably admitted the root but crashed before creating the control branch), the draft is not
/// admitted again, counts once against the graph's event limit, and the same admission is
/// returned with the unchanged graph.
///
/// # Errors
///
/// Returns `Campaign(MissingAdmission)` for another event kind, the Campaign error that
/// reconstruction raises for a malformed admission, `Campaign(CrossedIdentity)` when the supplied
/// identities are not the admission's, an error if the event draft is malformed,
/// `SharedControlGraph` naming the first other Campaign control event the graph holds under the
/// envelope, `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or
/// `GraphCapacityExhausted` when the root leaves no room for its stop, and an error if Choregraph
/// rejects the draft.
///
/// # Panics
///
/// Panics if the pinned history admission loses the event or changes graph limits.
// r[impl campaign.initial_admission.stop_reserve]
// r[impl campaign.initial_admission.root_retry]
pub fn admit_control_root(
    graph: &choregraph_history::HistoryGraph,
    control_event: campaign_core::CampaignControlEvent,
    projection_schema_id: campaign_core::Id,
    adapter_id: campaign_core::Id,
    control_domain_id: campaign_core::Id,
) -> crate::contracts::AdapterResult<crate::contracts::ControlRootAdmission> {
    let state = campaign_core::reconstruct(core::slice::from_ref(&control_event))?;
    let admitted = &state.admission.projection;
    if admitted.projection_schema_id != projection_schema_id
        || admitted.adapter_id != adapter_id
        || admitted.control_domain_id != control_domain_id
    {
        return Err(campaign_core::CampaignError::CrossedIdentity.into());
    }
    let draft = crate::envelope::control_event_draft(
        &control_event,
        projection_schema_id,
        adapter_id,
        control_domain_id,
        None,
    )?;
    let (held, is_held) = crate::admission::held_draft(graph, &draft);
    crate::occupancy::check_root_admission(graph, admitted, held)?;
    crate::headroom::check_root(&state, graph, is_held)?;
    let (history_event_id, graph_after_admission) = if is_held {
        (held, graph.clone())
    } else {
        let admission = graph.admit(draft)?;
        (admission.event, admission.graph)
    };
    assert!(graph_after_admission.event(history_event_id).is_some());
    assert_eq!(graph.limits(), graph_after_admission.limits());
    Ok(crate::contracts::ControlRootAdmission {
        envelope: crate::contracts::ControlPayloadEnvelope {
            history_event_id,
            control_event,
        },
        graph_after_admission,
    })
}
