const ROLE_UNFOLD: u32 = 1;
const ROLE_RESERVE: u32 = 2;
const ROLE_PLAN: u32 = 3;
const ROLE_ADMIT: u32 = 4;
const ROLE_SCHEDULE: u32 = 5;
const ROLE_COMMIT: u32 = 6;
const LABEL_ROLE_PROJECTION: &[u8] = b"frontier-role-projection";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.fd.roles]
pub enum FrontierRoleKind {
    /// Public Campaign core API variant.
    Unfold,
    /// Public Campaign core API variant.
    Reserve,
    /// Public Campaign core API variant.
    Plan,
    /// Public Campaign core API variant.
    Admit,
    /// Public Campaign core API variant.
    Schedule,
    /// Public Campaign core API variant.
    Commit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct FrontierRoleIdentity {
    /// Public Campaign core API value.
    pub role: FrontierRoleKind,
    /// Public Campaign core API value.
    pub object_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub role_id: crate::identity::Id,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Projection of one frontier state onto the six distinct frontier roles.
// r[impl campaign.fd.roles]
pub struct FrontierRoleProjection {
    /// Content-derived identity of the complete role projection.
    pub role_projection_id: crate::identity::Id,
    /// Unfold: stable discoveries of child moments.
    pub unfolding: alloc::vec::Vec<FrontierRoleIdentity>,
    /// Reserve: child reservations bound by accepted results.
    pub reservations: alloc::vec::Vec<FrontierRoleIdentity>,
    /// Plan: retained candidates with guidance, eligible for planning.
    pub plans: alloc::vec::Vec<FrontierRoleIdentity>,
    /// Admit: every admitted candidate record.
    pub admissions: alloc::vec::Vec<FrontierRoleIdentity>,
    /// Schedule: outstanding selections (queued work items).
    pub schedules: alloc::vec::Vec<FrontierRoleIdentity>,
    /// Commit: accepted results (computed results).
    pub commitments: alloc::vec::Vec<FrontierRoleIdentity>,
}

/// Projects the frontier onto its distinct roles and derives the role projection identity.
///
/// # Errors
///
/// Returns a typed `CampaignError` when a role identity cannot bind its role and object identity.
// r[impl campaign.fd.roles]
pub fn project_frontier_roles(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<FrontierRoleProjection> {
    let mut projection = FrontierRoleProjection {
        role_projection_id: crate::identity::Id::absent(),
        unfolding: id_roles(
            FrontierRoleKind::Unfold,
            state
                .discoveries
                .iter()
                .map(|discovery| discovery.discovery_id),
        )?,
        reservations: id_roles(
            FrontierRoleKind::Reserve,
            state.reservations.iter().copied(),
        )?,
        plans: id_roles(
            FrontierRoleKind::Plan,
            state
                .candidates
                .iter()
                .filter(|candidate| {
                    matches!(candidate.status, crate::model::CandidateStatus::Retained)
                        && candidate.latest_guidance.is_some()
                })
                .map(|candidate| candidate.candidate.candidate_id),
        )?,
        admissions: id_roles(
            FrontierRoleKind::Admit,
            state
                .candidates
                .iter()
                .map(|candidate| candidate.candidate.candidate_id),
        )?,
        schedules: id_roles(
            FrontierRoleKind::Schedule,
            state
                .outstanding
                .iter()
                .map(|selection| selection.selection_id),
        )?,
        commitments: id_roles(
            FrontierRoleKind::Commit,
            state.accepted_results.iter().copied(),
        )?,
    };
    projection.role_projection_id = derive_role_projection_id(&projection)?;
    debug_assert_eq!(
        projection.role_projection_id.kind,
        crate::identity::IdKind::RoleProjection
    );
    debug_assert_eq!(projection.admissions.len(), state.candidates.len());
    Ok(projection)
}

/// Derives the identity of a role projection from every role identity in role order.
///
/// # Errors
///
/// Returns a typed `CampaignError` when a role list length cannot be framed.
pub fn derive_role_projection_id(
    projection: &FrontierRoleProjection,
) -> crate::error::CampaignResult<crate::identity::Id> {
    crate::identity::derive_id_with(
        crate::identity::IdKind::RoleProjection,
        LABEL_ROLE_PROJECTION,
        |writer| {
            let sections = [
                &projection.unfolding,
                &projection.reservations,
                &projection.plans,
                &projection.admissions,
                &projection.schedules,
                &projection.commitments,
            ];
            let mut writer = writer;
            for section in sections {
                writer = writer.seq_len(crate::identity::usize_to_u64(section.len())?);
                for role in section {
                    writer = writer.id(role.role_id);
                }
            }
            Ok(writer)
        },
    )
}

fn id_roles(
    role: FrontierRoleKind,
    object_ids: impl Iterator<Item = crate::identity::Id>,
) -> crate::error::CampaignResult<alloc::vec::Vec<FrontierRoleIdentity>> {
    let mut roles = alloc::vec::Vec::with_capacity(object_ids.size_hint().0);
    for object_id in object_ids {
        roles.push(role_identity(role, object_id)?);
    }
    Ok(roles)
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct RoleCollapseFixture {
    /// Public Campaign core API value.
    pub candidate_record_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub queued_work_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub computed_result_id: crate::identity::Id,
}

/// Checks that an adapter's unvisited candidate, queued work item, and computed result keep
/// distinct identities.
///
/// # Errors
///
/// Returns `MissingIdentity` for an absent identity, `RoleCollapse` when two roles share one
/// identity digest (including one record reused for all three), and `CrossedIdentity` when a
/// distinct identity has the wrong kind for its role.
// r[impl campaign.fd.roles.collapsed]
pub fn check_no_role_collapse(fixture: &RoleCollapseFixture) -> crate::error::CampaignResult<()> {
    let ids = [
        fixture.candidate_record_id,
        fixture.queued_work_id,
        fixture.computed_result_id,
    ];
    if ids.iter().any(|id| id.is_absent()) {
        return Err(crate::error::CampaignError::MissingIdentity);
    }
    let candidate_digest = fixture.candidate_record_id.digest;
    let selection_digest = fixture.queued_work_id.digest;
    let result_digest = fixture.computed_result_id.digest;
    if candidate_digest == selection_digest
        || candidate_digest == result_digest
        || selection_digest == result_digest
    {
        return Err(crate::error::CampaignError::RoleCollapse);
    }
    fixture
        .candidate_record_id
        .require_kind(crate::identity::IdKind::Candidate)?;
    fixture
        .queued_work_id
        .require_kind(crate::identity::IdKind::Selection)?;
    fixture
        .computed_result_id
        .require_kind(crate::identity::IdKind::Result)?;
    debug_assert_ne!(candidate_digest, selection_digest);
    debug_assert_ne!(result_digest, candidate_digest);
    debug_assert_ne!(result_digest, selection_digest);
    Ok(())
}

fn role_identity(
    role: FrontierRoleKind,
    object_id: crate::identity::Id,
) -> crate::error::CampaignResult<FrontierRoleIdentity> {
    let role_id = crate::identity::derive_id_with(
        crate::identity::IdKind::DerivedRecord,
        b"frontier-role",
        |writer| {
            let writer = writer.u32(role_code(role));
            Ok(writer.id(object_id))
        },
    )?;
    Ok(FrontierRoleIdentity {
        role,
        object_id,
        role_id,
    })
}

#[cfg_attr(
    dylint_lib = "octet",
    allow(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "POLICY owns the closed role framing vocabulary; a new role needs an explicit distinct stable wire code"
    )
)]
const fn role_code(role: FrontierRoleKind) -> u32 {
    match role {
        FrontierRoleKind::Unfold => ROLE_UNFOLD,
        FrontierRoleKind::Reserve => ROLE_RESERVE,
        FrontierRoleKind::Plan => ROLE_PLAN,
        FrontierRoleKind::Admit => ROLE_ADMIT,
        FrontierRoleKind::Schedule => ROLE_SCHEDULE,
        FrontierRoleKind::Commit => ROLE_COMMIT,
    }
}
