//! The campaign admission a session opens with and Campaign's cold start of its control root.

/// Campaign admission bound to the root moment and the real pre-admission branch value.
pub(super) fn admission(
    identities: &crate::port::AdapterIdentities,
    root: crate::port::MomentIdentity,
    projection: campaign_core::ProjectionBinding,
    bounds: campaign_core::ExplorationBounds,
    budget_units: u64,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignAdmission> {
    let crate::port::MomentIdentity::Root { moment_id } = root else {
        return Err(crate::verdicts::ConformanceError::InvalidIdentities);
    };
    let admission = campaign_core::CampaignAdmission {
        campaign_id: campaign_core::Id::absent(),
        schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
        policy_id: crate::harness::suite_id(campaign_core::IdKind::Policy, "policy")?,
        guidance_profile_id: identities.guidance_profile_id,
        entropy_profile_id: identities.entropy_source_id,
        budget_profile_id: bounds.bounds_id,
        adapter_id: identities.adapter_id,
        control_domain_id: identities.control_domain_id,
        bounds,
        resource_budgets: identities
            .resource_dimension_ids
            .iter()
            .map(|dimension_id| campaign_core::ResourceAmount {
                dimension_id: *dimension_id,
                amount: budget_units,
            })
            .collect(),
        roots: alloc::vec![campaign_core::RootMoment {
            moment_id,
            projection_id: projection.projection_id,
            adapter_id: identities.adapter_id,
        }],
        operation_schema_ids: identities.operation_schema_ids.clone(),
        projection,
    };
    debug_assert_eq!(admission.adapter_id, identities.adapter_id);
    debug_assert_eq!(admission.roots.len(), 1);
    Ok(admission)
}

/// Campaign's cold-start validation of one history: the only load that holds no projection.
pub(super) fn cold_start(
    identities: &crate::port::AdapterIdentities,
    graph: &campaign_choregraph::HistoryGraph,
    branch: &campaign_choregraph::BranchRef,
    payloads: &[campaign_choregraph::ControlPayloadEnvelope],
) -> crate::verdicts::ConformanceResult<campaign_choregraph::ValidatedControlProjection> {
    let choices = campaign_choregraph::ParentChoices::new();
    let expected_binding = campaign_choregraph::derive_projection_binding(
        graph,
        branch,
        identities.projection_schema_id,
        identities.adapter_id,
        identities.control_domain_id,
    )?;
    let projection = campaign_choregraph::validate_control_projection(
        &campaign_choregraph::ControlProjectionRequest {
            graph,
            branch,
            parent_choices: &choices,
            maximum_events: crate::harness::HISTORY_MAX_EVENTS,
            expected_binding,
            payloads,
        },
    )?;
    debug_assert_eq!(projection.control_events().len(), payloads.len());
    debug_assert_eq!(projection.binding().generation, branch.generation);
    Ok(projection)
}
