const LABEL_PRUNE: &[u8] = b"prune-plan";

const LABEL_PRUNE_INVALIDATION: &[u8] = b"frontier-prune-invalidation";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Exact current-state request for the deterministic retention plan of a declared profile.
pub struct RetentionPlanRequest {
    /// Campaign whose frontier is planned.
    pub campaign_id: crate::identity::Id,
    /// Current frontier state the plan is computed from.
    pub state_id: crate::identity::Id,
    /// Declared retention profile identity.
    pub retention_profile_id: crate::identity::Id,
    /// Declared rank-dimension indices, highest priority first; unique and within the rank bound.
    pub rank_order: alloc::vec::Vec<u32>,
    /// Bounded pruning reason identity.
    pub reason_id: crate::identity::Id,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct PruneRequest {
    /// Public Campaign core API value.
    pub prune_plan_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub candidates: alloc::vec::Vec<crate::identity::Id>,
    /// Public Campaign core API value.
    pub reason_id: crate::identity::Id,
    /// Declared retention profile whose plan the targets must equal; `None` for an explicit caller list.
    pub retention_profile_id: Option<crate::identity::Id>,
    /// Declared rank order of the retention profile; empty for an explicit caller list.
    pub rank_order: alloc::vec::Vec<u32>,
    /// Public Campaign core API value.
    pub claims_artifact_deletion: bool,
    /// Public Campaign core API value.
    pub claims_history_rewrite: bool,
    /// Public Campaign core API value.
    pub claims_safe_deletion: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct PruneEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub pruned_candidates: alloc::vec::Vec<crate::identity::Id>,
    /// Public Campaign core API value.
    pub prune_plan_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub reason_id: crate::identity::Id,
    /// Declared retention profile of the plan, if any.
    pub retention_profile_id: Option<crate::identity::Id>,
    /// Declared rank order of the plan; empty without a retention profile.
    pub rank_order: alloc::vec::Vec<u32>,
    /// Public Campaign core API value.
    pub invalidation_id: crate::identity::Id,
}

impl PruneEvent {
    /// Reports the prune as an explicit invalidation of its plan targets, never as an addition.
    #[must_use]
    // r[impl campaign.fd.dynamics.invalidate]
    pub const fn dynamics(&self) -> crate::discipline::FrontierDynamics {
        crate::discipline::FrontierDynamics::Invalidation {
            invalidation_id: self.invalidation_id,
            cleared_dependency_id: self.prune_plan_id,
        }
    }
}

/// Derives the invalidation identity that a prune event carries.
///
/// # Errors
///
/// Returns a typed `CampaignError` when the plan or reason identity has the wrong kind.
pub fn derive_prune_invalidation_id(
    prune_plan_id: crate::identity::Id,
    reason_id: crate::identity::Id,
) -> crate::error::CampaignResult<crate::identity::Id> {
    prune_plan_id.require_kind(crate::identity::IdKind::PrunePlan)?;
    reason_id.require_kind(crate::identity::IdKind::Reason)?;
    crate::identity::derive_id_with(
        crate::identity::IdKind::Invalidation,
        LABEL_PRUNE_INVALIDATION,
        |writer| Ok(writer.id(prune_plan_id).id(reason_id)),
    )
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
/// # Panics
///
/// Panics only if the canonical copy changes the candidate count or fails to sort.
pub fn derive_prune_plan_id(
    request: &PruneRequest,
) -> crate::error::CampaignResult<crate::identity::Id> {
    let mut candidates = crate::identity::sorted_copy(&request.candidates);
    assert_eq!(candidates.len(), request.candidates.len());
    assert!(candidates.is_sorted());
    crate::identity::derive_id_with(
        crate::identity::IdKind::PrunePlan,
        LABEL_PRUNE,
        |mut writer| {
            writer = writer.id(request.campaign_id);
            writer = writer.id(request.state_id);
            writer = writer.id(request.reason_id);
            writer = writer.option_id(request.retention_profile_id);
            writer = writer.seq_len(crate::identity::usize_to_u64(request.rank_order.len())?);
            for dimension in &request.rank_order {
                writer = writer.u32(*dimension);
            }
            writer = writer.bool(request.claims_artifact_deletion);
            writer = writer.bool(request.claims_history_rewrite);
            writer = writer.bool(request.claims_safe_deletion);
            writer = writer.seq_len(crate::identity::usize_to_u64(candidates.len())?);
            for candidate in candidates.drain(..) {
                writer = writer.id(candidate);
            }
            Ok(writer)
        },
    )
}
