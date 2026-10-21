#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.fd.dynamics]
pub enum FrontierDynamics {
    /// Public Campaign core API variant.
    Discovery {
        /// Stable discovery identity.
        discovery_id: crate::identity::Id,
    },
    /// Public Campaign core API variant.
    MonotoneRelaxation {
        /// Relaxation event identity.
        relaxation_id: crate::identity::Id,
        /// Whether the relaxation strictly improved the prior value.
        improved: bool,
    },
    /// Public Campaign core API variant.
    Invalidation {
        /// Explicit invalidation identity.
        invalidation_id: crate::identity::Id,
        /// Cleared dependency identity.
        cleared_dependency_id: crate::identity::Id,
    },
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when a dynamic lacks its required identity or violates its transition invariant.
// r[impl campaign.fd.dynamics]
pub fn validate_dynamics(dynamics: &FrontierDynamics) -> crate::error::CampaignResult<()> {
    match dynamics {
        FrontierDynamics::Discovery { discovery_id } => discovery_id
            .require_kind(crate::identity::IdKind::Discovery)
            .map(|_| ()),
        FrontierDynamics::MonotoneRelaxation {
            relaxation_id,
            improved,
        } => validate_relaxation(*relaxation_id, *improved),
        FrontierDynamics::Invalidation {
            invalidation_id,
            cleared_dependency_id,
        } => validate_invalidation(*invalidation_id, *cleared_dependency_id),
    }
}

fn validate_relaxation(
    relaxation_id: crate::identity::Id,
    improved: bool,
) -> crate::error::CampaignResult<()> {
    relaxation_id.require_kind(crate::identity::IdKind::Relaxation)?;
    if !improved {
        return Err(crate::error::CampaignError::NonImprovingRelaxation);
    }
    Ok(())
}

fn validate_invalidation(
    invalidation_id: crate::identity::Id,
    cleared_dependency_id: crate::identity::Id,
) -> crate::error::CampaignResult<()> {
    invalidation_id.require_kind(crate::identity::IdKind::Invalidation)?;
    require_invalidated_dependency_kind(cleared_dependency_id)
}

fn require_invalidated_dependency_kind(
    dependency_id: crate::identity::Id,
) -> crate::error::CampaignResult<()> {
    if dependency_id.is_absent() {
        return Err(crate::error::CampaignError::MissingIdentity);
    }
    let is_valid_kind = matches!(
        dependency_id.kind,
        crate::identity::IdKind::Candidate
            | crate::identity::IdKind::GuidanceRecord
            | crate::identity::IdKind::PrunePlan
    );
    if !is_valid_kind {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.fd.oracle]
pub enum PolicyImplementation {
    /// Public Campaign core API variant.
    ProductionCompiled {
        /// Compiled production policy identity.
        policy_id: crate::identity::Id,
    },
    /// Public Campaign core API variant.
    ReferenceEvidence {
        /// Reference-only evidence identity.
        reference_id: crate::identity::Id,
    },
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when the selected implementation is reference evidence or has an invalid policy identity.
// r[impl campaign.fd.oracle]
pub fn select_production_policy(
    selected: &PolicyImplementation,
) -> crate::error::CampaignResult<crate::identity::Id> {
    if let PolicyImplementation::ProductionCompiled { policy_id } = selected {
        return policy_id.require_kind(crate::identity::IdKind::Policy);
    }
    Err(crate::error::CampaignError::ReferencePolicySelected)
}
