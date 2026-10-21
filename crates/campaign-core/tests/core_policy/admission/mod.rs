#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) budget: u64,
    pub(crate) max_outstanding: u64,
}
pub(crate) fn xid(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    campaign_core::derive_id(kind, label, &[])
}
pub(crate) fn projection() -> Result<campaign_core::ProjectionBinding, String> {
    (campaign_core::ProjectionBinding {
        projection_id: campaign_core::Id::absent(),
        graph_id: xid(campaign_core::IdKind::Graph, b"graph"),
        control_branch_id: xid(campaign_core::IdKind::ControlBranch, b"control-branch"),
        control_branch_name_id: xid(campaign_core::IdKind::ControlBranch, b"control-branch-name"),
        generation: crate::suite::constants::GENERATION_CURRENT,
        head_id: xid(campaign_core::IdKind::Head, b"head"),
        projection_schema_id: xid(campaign_core::IdKind::Projection, b"projection-schema"),
        adapter_id: xid(campaign_core::IdKind::Adapter, b"adapter"),
        control_domain_id: xid(campaign_core::IdKind::ControlDomain, b"control-domain"),
    }
    .derive())
    .map_err(|error| format!("{error:?}"))
}
/// The state's current history binding: the admitted binding advanced one generation per control
/// event (the adapter's fast-forward rule), the only binding receipts accept for the state.
pub(crate) fn current_projection(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::ProjectionBinding, String> {
    let mut binding = state.admission.projection.clone();
    binding.projection_id = campaign_core::Id::absent();
    binding.generation = binding
        .generation
        .checked_add(state.control_event_count)
        .ok_or_else(|| String::from("generation overflow"))?;
    binding.derive().map_err(|error| format!("{error:?}"))
}
pub(crate) fn bounds(max_outstanding: u64) -> Result<campaign_core::ExplorationBounds, String> {
    (campaign_core::ExplorationBounds {
        bounds_id: campaign_core::Id::absent(),
        max_roots: crate::suite::constants::BOUND_MANY,
        max_candidates: crate::suite::constants::BOUND_MANY,
        max_candidate_batch: crate::suite::constants::BOUND_MANY,
        max_retained_candidates: crate::suite::constants::BOUND_MANY,
        max_selections: crate::suite::constants::BOUND_MANY,
        max_guidance_records: crate::suite::constants::BOUND_MANY,
        max_outstanding,
        max_depth: crate::suite::constants::BOUND_MANY,
        max_child_moments: crate::suite::constants::BOUND_MANY,
        max_derived_records: crate::suite::constants::BOUND_MANY,
        max_rank_dimensions: crate::suite::constants::BOUND_MANY,
        max_rank_magnitude: crate::suite::constants::RANK_MAGNITUDE,
        max_findings: crate::suite::constants::BOUND_MANY,
        max_control_events: crate::suite::constants::BOUND_MANY,
        max_resource_dimensions: crate::suite::constants::BOUND_MANY,
        max_operation_schemas: crate::suite::constants::BOUND_MANY,
        max_prune_batch: crate::suite::constants::BOUND_MANY,
    }
    .derive())
    .map_err(|error| format!("{error:?}"))
}
pub(crate) fn with_budget(
    limits: crate::suite::admission::Limits,
) -> Result<campaign_core::CampaignAdmission, String> {
    let Limits {
        budget,
        max_outstanding,
    } = limits;
    let projection = projection()?;
    assert_eq!(
        projection.projection_id.kind,
        campaign_core::IdKind::Projection
    );
    assert_ne!(projection.projection_id, campaign_core::Id::absent());
    Ok(campaign_core::CampaignAdmission {
        campaign_id: campaign_core::Id::absent(),
        schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
        projection: projection.clone(),
        policy_id: xid(campaign_core::IdKind::Policy, b"policy"),
        guidance_profile_id: xid(campaign_core::IdKind::GuidanceProfile, b"guidance-profile"),
        entropy_profile_id: xid(campaign_core::IdKind::EntropyProfile, b"entropy-profile"),
        budget_profile_id: xid(campaign_core::IdKind::BudgetProfile, b"budget-profile"),
        adapter_id: projection.adapter_id,
        control_domain_id: projection.control_domain_id,
        bounds: bounds(max_outstanding)?,
        resource_budgets: vec![campaign_core::ResourceAmount {
            dimension_id: xid(campaign_core::IdKind::ResourceDimension, b"resource"),
            amount: budget,
        }],
        roots: vec![campaign_core::RootMoment {
            moment_id: xid(campaign_core::IdKind::RootMoment, b"root"),
            projection_id: projection.projection_id,
            adapter_id: projection.adapter_id,
        }],
        operation_schema_ids: vec![xid(
            campaign_core::IdKind::OperationSchema,
            b"operation-schema",
        )],
    })
}
pub(crate) fn initial_state() -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    admitted(with_budget(crate::suite::admission::Limits {
        budget: crate::suite::constants::BUDGET_TOTAL,
        max_outstanding: crate::suite::constants::BOUND_MANY,
    })?)
}
pub(crate) fn default_input() -> Result<campaign_core::CampaignAdmission, String> {
    with_budget(crate::suite::admission::Limits {
        budget: crate::suite::constants::BUDGET_TOTAL,
        max_outstanding: crate::suite::constants::BOUND_MANY,
    })
}
pub(crate) fn admitted(
    input: campaign_core::CampaignAdmission,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    let event = (campaign_core::admit_campaign(input)).map_err(|error| format!("{error:?}"))?;
    let events = vec![event];
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    Ok((events, state))
}
