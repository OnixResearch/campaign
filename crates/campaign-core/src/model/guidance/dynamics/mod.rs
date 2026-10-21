//! Guidance dynamics: discovery, monotone relaxation, coalescing, and explicit revision.

const LABEL_GUIDANCE_OUTCOME: &[u8] = b"guidance-admission-outcome";
const LABEL_GUIDANCE_DISCOVERY: &[u8] = b"guidance-discovery";
const LABEL_GUIDANCE_RELAXATION: &[u8] = b"guidance-relaxation";
const LABEL_GUIDANCE_INVALIDATION: &[u8] = b"guidance-revision";
const DYNAMICS_DISCOVERY: u32 = 1;
const DYNAMICS_STRICT_IMPROVEMENT: u32 = 2;
const DYNAMICS_COALESCED_DUPLICATE: u32 = 3;
const DYNAMICS_REVISION: u32 = 4;

// r[impl campaign.fd.dynamics]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value: dynamic by which one guidance record entered the frontier.
pub enum GuidanceDynamics {
    /// First guidance record for its candidate.
    Discovery,
    /// Monotone relaxation: higher generation and strictly better lexicographic ranks.
    StrictImprovement {
        /// Previous guidance superseded by this strict improvement.
        previous_guidance_id: crate::identity::Id,
    },
    /// Equal ranks: the update is coalesced into the existing record and adds nothing.
    CoalescedDuplicate {
        /// Existing guidance matched by this duplicate update.
        existing_guidance_id: crate::identity::Id,
    },
    /// Explicit invalidation of the latest record by a successor with any valid ranks.
    Revision {
        /// Latest guidance record invalidated by the revision.
        invalidated_guidance_id: crate::identity::Id,
        /// Invalidation identity derived from the invalidated record and the reason.
        invalidation_id: crate::identity::Id,
        /// Bounded consumer reason identity.
        reason_id: crate::identity::Id,
    },
}

impl GuidanceDynamics {
    /// Classifies a relaxation-only update of the latest guidance record.
    ///
    /// # Errors
    ///
    /// Returns `StaleGuidance` when the generation does not increase, `NonImprovingRelaxation`
    /// when the ranks are neither equal nor strictly better, and `InvalidGuidanceShape` when
    /// the rank dimensions differ.
    // r[impl campaign.fd.dynamics.relax]
    // r[impl campaign.guidance.coalesced]
    // r[impl campaign.guidance.non_improving]
    // r[impl campaign.guidance.update]
    pub fn relaxation(
        previous: Option<&super::GuidanceRecord>,
        successor: &super::GuidanceRecord,
    ) -> crate::error::CampaignResult<Self> {
        let Some(old) = previous else {
            return Ok(Self::Discovery);
        };
        if old.guidance_id == successor.guidance_id || old.ranks == successor.ranks {
            if successor.generation < old.generation {
                return Err(crate::error::CampaignError::StaleGuidance);
            }
            return Ok(Self::CoalescedDuplicate {
                existing_guidance_id: old.guidance_id,
            });
        }
        if successor.generation <= old.generation {
            return Err(crate::error::CampaignError::StaleGuidance);
        }
        if !strictly_better(&successor.ranks, &old.ranks)? {
            return Err(crate::error::CampaignError::NonImprovingRelaxation);
        }
        debug_assert!(successor.generation > old.generation);
        Ok(Self::StrictImprovement {
            previous_guidance_id: old.guidance_id,
        })
    }

    /// Builds an explicit revision that invalidates the latest guidance record.
    ///
    /// # Errors
    ///
    /// Returns `MissingGuidance` without a latest record, `StaleGuidance` when the invalidated
    /// identity is not the latest record or the generation does not increase, and identity
    /// errors for a missing or crossed reason.
    // r[impl campaign.guidance.revision]
    pub fn revision(
        previous: Option<&super::GuidanceRecord>,
        successor: &super::GuidanceRecord,
        invalidated_guidance_id: crate::identity::Id,
        reason_id: crate::identity::Id,
    ) -> crate::error::CampaignResult<Self> {
        invalidated_guidance_id.require_kind(crate::identity::IdKind::GuidanceRecord)?;
        reason_id.require_kind(crate::identity::IdKind::Reason)?;
        let old = previous.ok_or(crate::error::CampaignError::MissingGuidance)?;
        if old.guidance_id != invalidated_guidance_id || successor.generation <= old.generation {
            return Err(crate::error::CampaignError::StaleGuidance);
        }
        let invalidation_id = derive_revision_invalidation_id(invalidated_guidance_id, reason_id)?;
        debug_assert_eq!(invalidation_id.kind, crate::identity::IdKind::Invalidation);
        Ok(Self::Revision {
            invalidated_guidance_id,
            invalidation_id,
            reason_id,
        })
    }

    /// Returns true when the dynamic adds a new latest guidance record.
    #[must_use]
    pub const fn adds_record(&self) -> bool {
        !matches!(self, Self::CoalescedDuplicate { .. })
    }

    /// Projects the dynamic onto the frontier-discipline vocabulary; coalescing has no dynamic.
    ///
    /// # Errors
    ///
    /// Returns `MissingGuidance` when a relaxation names no previous record and identity or
    /// shape errors when its identities cannot be derived.
    pub fn frontier_dynamics(
        &self,
        previous: Option<&super::GuidanceRecord>,
        successor: &super::GuidanceRecord,
    ) -> crate::error::CampaignResult<Option<crate::discipline::FrontierDynamics>> {
        match self {
            Self::Discovery => Ok(Some(crate::discipline::FrontierDynamics::Discovery {
                discovery_id: crate::identity::derive_id_with(
                    crate::identity::IdKind::Discovery,
                    LABEL_GUIDANCE_DISCOVERY,
                    |writer| Ok(writer.id(successor.guidance_id)),
                )?,
            })),
            Self::StrictImprovement {
                previous_guidance_id,
            } => relaxation_fact(previous, successor, *previous_guidance_id).map(Some),
            Self::CoalescedDuplicate { .. } => Ok(None),
            Self::Revision {
                invalidated_guidance_id,
                invalidation_id,
                ..
            } => Ok(Some(crate::discipline::FrontierDynamics::Invalidation {
                invalidation_id: *invalidation_id,
                cleared_dependency_id: *invalidated_guidance_id,
            })),
        }
    }

    /// Derives the `AdmissionOutcomes` identity recorded for one guidance event.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when identity framing fails.
    pub fn outcomes_id(
        &self,
        guidance_id: crate::identity::Id,
    ) -> crate::error::CampaignResult<crate::identity::Id> {
        crate::identity::derive_id_with(
            crate::identity::IdKind::AdmissionOutcomes,
            LABEL_GUIDANCE_OUTCOME,
            |writer| {
                let writer = writer.id(guidance_id);
                Ok(match self {
                    Self::Discovery => writer.u32(DYNAMICS_DISCOVERY),
                    Self::StrictImprovement {
                        previous_guidance_id,
                    } => writer
                        .u32(DYNAMICS_STRICT_IMPROVEMENT)
                        .id(*previous_guidance_id),
                    Self::CoalescedDuplicate {
                        existing_guidance_id,
                    } => writer
                        .u32(DYNAMICS_COALESCED_DUPLICATE)
                        .id(*existing_guidance_id),
                    Self::Revision {
                        invalidated_guidance_id,
                        invalidation_id,
                        reason_id,
                    } => writer
                        .u32(DYNAMICS_REVISION)
                        .id(*invalidated_guidance_id)
                        .id(*invalidation_id)
                        .id(*reason_id),
                })
            },
        )
    }
}

/// Public Campaign core API operation: derives the `Invalidation` identity of a guidance revision.
///
/// # Errors
///
/// Returns a typed `CampaignError` when identity framing fails.
pub fn derive_revision_invalidation_id(
    invalidated_guidance_id: crate::identity::Id,
    reason_id: crate::identity::Id,
) -> crate::error::CampaignResult<crate::identity::Id> {
    crate::identity::derive_id_with(
        crate::identity::IdKind::Invalidation,
        LABEL_GUIDANCE_INVALIDATION,
        |writer| Ok(writer.id(invalidated_guidance_id).id(reason_id)),
    )
}

fn relaxation_fact(
    previous: Option<&super::GuidanceRecord>,
    successor: &super::GuidanceRecord,
    previous_guidance_id: crate::identity::Id,
) -> crate::error::CampaignResult<crate::discipline::FrontierDynamics> {
    let old = previous.ok_or(crate::error::CampaignError::MissingGuidance)?;
    let is_improved =
        successor.generation > old.generation && strictly_better(&successor.ranks, &old.ranks)?;
    let relaxation_id = crate::identity::derive_id_with(
        crate::identity::IdKind::Relaxation,
        LABEL_GUIDANCE_RELAXATION,
        |writer| Ok(writer.id(previous_guidance_id).id(successor.guidance_id)),
    )?;
    Ok(crate::discipline::FrontierDynamics::MonotoneRelaxation {
        relaxation_id,
        improved: is_improved,
    })
}

fn strictly_better(new: &[i64], old: &[i64]) -> crate::error::CampaignResult<bool> {
    if new.len() != old.len() {
        return Err(crate::error::CampaignError::InvalidGuidanceShape);
    }
    for (new_rank, old_rank) in new.iter().zip(old.iter()) {
        if new_rank != old_rank {
            return Ok(new_rank > old_rank);
        }
    }
    Ok(false)
}
