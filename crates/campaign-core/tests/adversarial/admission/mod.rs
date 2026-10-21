#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) resource_budget: u64,
    pub(crate) max_findings: u64,
}
pub(crate) fn tid(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    campaign_core::derive_id(kind, label, &[])
}
pub(crate) fn projection_binding() -> Result<campaign_core::ProjectionBinding, String> {
    (campaign_core::ProjectionBinding {
        projection_id: campaign_core::Id::absent(),
        graph_id: tid(campaign_core::IdKind::Graph, b"adversarial-graph"),
        control_branch_id: tid(
            campaign_core::IdKind::ControlBranch,
            b"adversarial-control-branch",
        ),
        control_branch_name_id: tid(
            campaign_core::IdKind::ControlBranch,
            b"adversarial-control-branch-name",
        ),
        generation: crate::suite::constants::CURRENT_GENERATION,
        head_id: tid(campaign_core::IdKind::Head, b"adversarial-head"),
        projection_schema_id: tid(
            campaign_core::IdKind::Projection,
            b"adversarial-projection-schema",
        ),
        adapter_id: tid(campaign_core::IdKind::Adapter, b"adversarial-adapter"),
        control_domain_id: tid(
            campaign_core::IdKind::ControlDomain,
            b"adversarial-control-domain",
        ),
    }
    .derive())
    .map_err(|error| format!("{error:?}"))
}
pub(crate) fn bounds_with_findings(
    max_findings: u64,
) -> Result<campaign_core::ExplorationBounds, String> {
    (campaign_core::ExplorationBounds {
        bounds_id: campaign_core::Id::absent(),
        max_roots: crate::suite::constants::BOUND_MANY,
        max_candidates: crate::suite::constants::BOUND_MANY,
        max_candidate_batch: crate::suite::constants::BOUND_MANY,
        max_retained_candidates: crate::suite::constants::BOUND_MANY,
        max_selections: crate::suite::constants::BOUND_MANY,
        max_guidance_records: crate::suite::constants::BOUND_MANY,
        max_outstanding: crate::suite::constants::BOUND_MANY,
        max_depth: crate::suite::constants::BOUND_MANY,
        max_child_moments: crate::suite::constants::BOUND_MANY,
        max_derived_records: crate::suite::constants::BOUND_MANY,
        max_rank_dimensions: crate::suite::constants::BOUND_MANY,
        max_rank_magnitude: crate::suite::constants::RANK_MAGNITUDE,
        max_findings,
        max_control_events: crate::suite::constants::BOUND_MANY,
        max_resource_dimensions: crate::suite::constants::BOUND_MANY,
        max_operation_schemas: crate::suite::constants::BOUND_MANY,
        max_prune_batch: crate::suite::constants::BOUND_MANY,
    }
    .derive())
    .map_err(|error| format!("{error:?}"))
}
pub(crate) fn configure(
    limits: crate::suite::admission::Limits,
) -> Result<campaign_core::CampaignAdmission, String> {
    let Limits {
        resource_budget,
        max_findings,
    } = limits;
    let projection = projection_binding()?;
    assert_eq!(
        projection.projection_id.kind,
        campaign_core::IdKind::Projection
    );
    assert_ne!(projection.projection_id, campaign_core::Id::absent());
    Ok(campaign_core::CampaignAdmission {
        campaign_id: campaign_core::Id::absent(),
        schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
        projection: projection.clone(),
        policy_id: tid(campaign_core::IdKind::Policy, b"adversarial-policy"),
        guidance_profile_id: tid(
            campaign_core::IdKind::GuidanceProfile,
            b"adversarial-guidance-profile",
        ),
        entropy_profile_id: tid(
            campaign_core::IdKind::EntropyProfile,
            b"adversarial-entropy-profile",
        ),
        budget_profile_id: tid(
            campaign_core::IdKind::BudgetProfile,
            b"adversarial-budget-profile",
        ),
        adapter_id: projection.adapter_id,
        control_domain_id: projection.control_domain_id,
        bounds: bounds_with_findings(max_findings)?,
        resource_budgets: vec![campaign_core::ResourceAmount {
            dimension_id: tid(
                campaign_core::IdKind::ResourceDimension,
                b"adversarial-resource",
            ),
            amount: resource_budget,
        }],
        roots: vec![campaign_core::RootMoment {
            moment_id: tid(campaign_core::IdKind::RootMoment, b"adversarial-root"),
            projection_id: projection.projection_id,
            adapter_id: projection.adapter_id,
        }],
        operation_schema_ids: vec![tid(
            campaign_core::IdKind::OperationSchema,
            b"adversarial-operation-schema",
        )],
    })
}
pub(crate) fn admitted_state(
    limits: crate::suite::admission::Limits,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    let Limits {
        resource_budget,
        max_findings,
    } = limits;
    let event = (campaign_core::admit_campaign(configure(crate::suite::admission::Limits {
        resource_budget,
        max_findings,
    })?))
    .map_err(|error| format!("{error:?}"))?;
    let events = vec![event];
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    Ok((events, state))
}
