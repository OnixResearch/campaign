//! Derived-cache validation against authoritative Choregraph history.

/// Authoritative history and host configuration against which a derived cache is validated.
pub struct CacheValidationRequest<'a> {
    /// The current immutable Choregraph graph.
    pub graph: &'a choregraph_history::HistoryGraph,
    /// The current generation-fenced control branch.
    pub branch: &'a choregraph_history::BranchRef,
    /// Explicit parent choices for every multi-parent history event.
    pub parent_choices: &'a choregraph_history::ParentChoices,
    /// Maximum events admitted to the Choregraph linear projection.
    pub maximum_events: u32,
    /// Host-configured Campaign projection schema identity.
    pub projection_schema_id: campaign_core::Id,
    /// Host-configured Campaign adapter identity.
    pub adapter_id: campaign_core::Id,
    /// Host-configured Campaign control-domain identity.
    pub control_domain_id: campaign_core::Id,
    /// Control payloads in the same order as the selected linear projection.
    pub payloads: &'a [crate::contracts::ControlPayloadEnvelope],
    /// The projection the host holds, if any. When present, the history is validated as its
    /// fast-forward through [`crate::validate_control_advance`], so a cache of a branch moved
    /// onto a fork of the held lineage never validates; `None` is a cold-start load through
    /// [`crate::validate_control_projection`].
    pub previous: Option<&'a crate::contracts::ValidatedControlProjection>,
}

/// Validates a derived cache against the binding freshly derived from the graph and branch and
/// the Campaign state reconstructed from the authoritative control history, as a fast-forward of
/// the held projection when the request carries one.
///
/// Returns the authoritative reconstructed state the cache stands for.
///
/// # Errors
///
/// Returns `Campaign(InvalidCache)` when the cache binds a stale head, generation, graph, branch,
/// policy, adapter, state, or event count, `NonFastForwardBranch` when the history does not
/// extend the held projection, `Campaign(CrossedIdentity)` when the host-configured identities are
/// not the admission's, and the projection error when the history itself is not an exact
/// Campaign control projection.
// r[impl campaign.cache]
pub fn validate_cache_against_history(
    request: &CacheValidationRequest<'_>,
    cache: &campaign_core::CacheProjection,
) -> crate::contracts::AdapterResult<campaign_core::FrontierState> {
    let fresh_binding = crate::identity::derive_projection_binding(
        request.graph,
        request.branch,
        request.projection_schema_id,
        request.adapter_id,
        request.control_domain_id,
    )?;
    if cache.binding != fresh_binding {
        return Err(campaign_core::CampaignError::InvalidCache.into());
    }
    let projection = crate::contracts::ControlProjectionRequest {
        graph: request.graph,
        branch: request.branch,
        parent_choices: request.parent_choices,
        maximum_events: request.maximum_events,
        expected_binding: fresh_binding,
        payloads: request.payloads,
    };
    let validated = match request.previous {
        Some(previous) => crate::fast_forward::validate_control_advance(previous, &projection)?,
        None => crate::lineage::validate_control_projection(&projection)?,
    };
    campaign_core::validate_cache_for_state(cache, &validated.binding, &validated.state)?;
    debug_assert_eq!(cache.state_id, validated.state.state_id);
    Ok(validated.state)
}
