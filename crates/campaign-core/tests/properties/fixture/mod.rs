//! Explicit identities and inputs built only through public constructors and `derive`.

/// Largest rank a generated guidance record may carry; small so relaxations can reach it.
pub(crate) const RANK_LIMIT: i64 = 64;
/// Rank magnitude bound declared by every property campaign.
pub(crate) const RANK_MAGNITUDE: u64 = RANK_LIMIT.unsigned_abs();
/// Bound for dimensions the generated shapes never stress.
pub(crate) const BOUND_WIDE: u64 = 16;
/// Projection generation every property campaign is admitted at.
pub(crate) const GENERATION: u64 = 3;
/// First guidance generation for a candidate.
pub(crate) const FIRST_GENERATION: u64 = 1;

/// Identity of `kind` derived from a fixed tag and a numeric discriminator.
pub(crate) fn id(kind: campaign_core::IdKind, tag: &[u8], index: u64) -> campaign_core::Id {
    let mut label = tag.to_vec();
    label.extend_from_slice(&index.to_le_bytes());
    campaign_core::derive_id(kind, &label, &[])
}

/// Identity of `kind` derived from a fixed tag and one bound identity.
pub(crate) fn bound_id(
    kind: campaign_core::IdKind,
    tag: &[u8],
    field: campaign_core::Id,
) -> campaign_core::Id {
    campaign_core::derive_id(kind, tag, &[field])
}

/// Position `pick` wraps to inside a non-empty collection of `count` items.
pub(crate) fn position(pick: u8, count: usize) -> Result<usize, String> {
    let wrapped = usize::from(pick)
        .checked_rem(count)
        .ok_or_else(|| String::from("cannot pick from an empty collection"))?;
    debug_assert!(wrapped < count);
    Ok(wrapped)
}

pub(crate) fn observed<T>(value: campaign_core::CampaignResult<T>) -> Result<T, String> {
    value.map_err(|error| format!("{error:?}"))
}

pub(crate) fn denied<T: core::fmt::Debug>(
    value: campaign_core::CampaignResult<T>,
) -> Result<campaign_core::CampaignError, String> {
    match value {
        Ok(accepted) => Err(format!("expected a typed denial, got {accepted:?}")),
        Err(error) => Ok(error),
    }
}

pub(crate) fn projection() -> Result<campaign_core::ProjectionBinding, String> {
    observed(
        campaign_core::ProjectionBinding {
            projection_id: campaign_core::Id::absent(),
            graph_id: id(campaign_core::IdKind::Graph, b"graph", 0),
            control_branch_id: id(campaign_core::IdKind::ControlBranch, b"branch", 0),
            control_branch_name_id: id(campaign_core::IdKind::ControlBranch, b"branch-name", 0),
            generation: GENERATION,
            head_id: id(campaign_core::IdKind::Head, b"head", 0),
            projection_schema_id: id(campaign_core::IdKind::Projection, b"schema", 0),
            adapter_id: id(campaign_core::IdKind::Adapter, b"adapter", 0),
            control_domain_id: id(campaign_core::IdKind::ControlDomain, b"domain", 0),
        }
        .derive(),
    )
}

pub(crate) fn dimension(index: u64) -> campaign_core::Id {
    id(
        campaign_core::IdKind::ResourceDimension,
        b"dimension",
        index,
    )
}

pub(crate) fn schema(index: u64) -> campaign_core::Id {
    id(campaign_core::IdKind::OperationSchema, b"schema", index)
}

pub(crate) fn root(
    projection: &campaign_core::ProjectionBinding,
    index: u64,
) -> campaign_core::RootMoment {
    campaign_core::RootMoment {
        moment_id: id(campaign_core::IdKind::RootMoment, b"root", index),
        projection_id: projection.projection_id,
        adapter_id: projection.adapter_id,
    }
}

/// Exploration bounds for one generated campaign shape.
#[derive(Clone, Debug)]
pub(crate) struct Limits {
    pub(crate) max_outstanding: u64,
    pub(crate) max_candidates: u64,
    pub(crate) max_candidate_batch: u64,
    pub(crate) max_selections: u64,
    pub(crate) max_guidance_records: u64,
    pub(crate) max_depth: u64,
    pub(crate) max_control_events: u64,
    pub(crate) rank_dimensions: usize,
}

pub(crate) fn bounds(limits: &Limits) -> Result<campaign_core::ExplorationBounds, String> {
    let max_rank_dimensions =
        u64::try_from(limits.rank_dimensions).map_err(|error| format!("{error}"))?;
    debug_assert!(max_rank_dimensions > 0);
    debug_assert!(limits.max_outstanding > 0);
    observed(
        campaign_core::ExplorationBounds {
            bounds_id: campaign_core::Id::absent(),
            max_roots: BOUND_WIDE,
            max_candidates: limits.max_candidates,
            max_candidate_batch: limits.max_candidate_batch,
            max_retained_candidates: BOUND_WIDE,
            max_selections: limits.max_selections,
            max_guidance_records: limits.max_guidance_records,
            max_outstanding: limits.max_outstanding,
            max_depth: limits.max_depth,
            max_child_moments: BOUND_WIDE,
            max_derived_records: BOUND_WIDE,
            max_rank_dimensions,
            max_rank_magnitude: RANK_MAGNITUDE,
            max_findings: BOUND_WIDE,
            max_control_events: limits.max_control_events,
            max_resource_dimensions: BOUND_WIDE,
            max_operation_schemas: BOUND_WIDE,
            max_prune_batch: BOUND_WIDE,
        }
        .derive(),
    )
}

/// Underived admission input listing roots, budgets, and schemas in the given order.
pub(crate) fn admission(
    bounds: campaign_core::ExplorationBounds,
    roots: Vec<campaign_core::RootMoment>,
    resource_budgets: Vec<campaign_core::ResourceAmount>,
    operation_schema_ids: Vec<campaign_core::Id>,
) -> Result<campaign_core::CampaignAdmission, String> {
    let projection = projection()?;
    Ok(campaign_core::CampaignAdmission {
        campaign_id: campaign_core::Id::absent(),
        schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
        policy_id: id(campaign_core::IdKind::Policy, b"policy", 0),
        guidance_profile_id: id(campaign_core::IdKind::GuidanceProfile, b"guidance", 0),
        entropy_profile_id: id(campaign_core::IdKind::EntropyProfile, b"entropy", 0),
        budget_profile_id: id(campaign_core::IdKind::BudgetProfile, b"budget", 0),
        adapter_id: projection.adapter_id,
        control_domain_id: projection.control_domain_id,
        projection,
        bounds,
        resource_budgets,
        roots,
        operation_schema_ids,
    })
}

pub(crate) fn campaign(
    input: campaign_core::CampaignAdmission,
) -> Result<
    (
        campaign_core::CampaignControlEvent,
        campaign_core::FrontierState,
    ),
    String,
> {
    let event = observed(campaign_core::admit_campaign(input))?;
    let state = observed(campaign_core::reconstruct(core::slice::from_ref(&event)))?;
    Ok((event, state))
}

pub(crate) fn rule(coalesce: bool) -> campaign_core::AdmissionRule {
    campaign_core::AdmissionRule {
        duplicate_candidate: if coalesce {
            campaign_core::DuplicateCandidateRule::Coalesce
        } else {
            campaign_core::DuplicateCandidateRule::Reject
        },
    }
}

pub(crate) fn coalescing_policy() -> campaign_core::Id {
    id(campaign_core::IdKind::CoalescingPolicy, b"coalesce", 0)
}

/// Underived candidate record `label` under `parent`.
pub(crate) fn record(
    state: &campaign_core::FrontierState,
    label: u64,
    parent: campaign_core::KnownMoment,
    declared_costs: Vec<campaign_core::ResourceAmount>,
) -> campaign_core::CandidateRecord {
    campaign_core::CandidateRecord {
        candidate_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        parent_moment_id: parent.moment_id,
        operation_profile_id: id(campaign_core::IdKind::OperationProfile, b"operation", label),
        operation_schema_id: schema(0),
        source_adapter_id: state.admission.adapter_id,
        depth: parent.depth.saturating_add(1),
        cost_profile_id: id(campaign_core::IdKind::CostProfile, b"cost", label),
        declared_costs,
        operation_generation_profile_id: id(campaign_core::IdKind::Policy, b"generation", 0),
    }
}

pub(crate) fn guidance(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    generation: u64,
    ranks: Vec<i64>,
) -> campaign_core::GuidanceRecord {
    campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation,
        ranks,
        source_facts_id: id(campaign_core::IdKind::SourceFacts, b"facts", generation),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    }
}

/// Declared costs for every positive per-dimension amount; one unit when all are zero.
pub(crate) fn costs(amounts: &[u64]) -> Vec<campaign_core::ResourceAmount> {
    let mut declared: Vec<campaign_core::ResourceAmount> = (0_u64..)
        .zip(amounts)
        .filter(|(_, amount)| **amount > 0)
        .map(|(index, amount)| campaign_core::ResourceAmount {
            dimension_id: dimension(index),
            amount: *amount,
        })
        .collect();
    if declared.is_empty() {
        declared.push(campaign_core::ResourceAmount {
            dimension_id: dimension(0),
            amount: campaign_core::UNIT_COST,
        });
    }
    declared
}
