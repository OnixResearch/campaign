//! `campaign.fd.roles` controls over a populated frontier.

fn objects(roles: &[campaign_core::FrontierRoleIdentity]) -> Vec<campaign_core::Id> {
    roles.iter().map(|role| role.object_id).collect()
}

fn sorted(mut ids: Vec<campaign_core::Id>) -> Vec<campaign_core::Id> {
    ids.sort();
    ids
}

/// Asserts every section is non-empty, labelled with its role, and that no role id repeats.
fn assert_distinct_sections(roles: &campaign_core::FrontierRoleProjection) {
    let sections = [
        (&roles.unfolding, campaign_core::FrontierRoleKind::Unfold),
        (
            &roles.reservations,
            campaign_core::FrontierRoleKind::Reserve,
        ),
        (&roles.plans, campaign_core::FrontierRoleKind::Plan),
        (&roles.admissions, campaign_core::FrontierRoleKind::Admit),
        (&roles.schedules, campaign_core::FrontierRoleKind::Schedule),
        (&roles.commitments, campaign_core::FrontierRoleKind::Commit),
    ];
    let total = sections.iter().map(|(section, _)| section.len()).sum();
    let mut role_ids = Vec::with_capacity(total);
    for (section, kind) in sections {
        assert!(!section.is_empty());
        assert!(section.iter().all(|role| role.role == kind));
        role_ids.extend(section.iter().map(|role| role.role_id));
    }
    role_ids.sort();
    role_ids.dedup();
    assert_eq!(role_ids.len(), total);
}

#[test]
// r[verify campaign.fd.roles.distinct]
fn every_role_of_one_moment_stays_separately_identifiable() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let root = world.state.admission.roots[0].moment_id;
    let candidate = crate::suite::world::observed(world.state.candidate(world.candidate_id))?;
    assert_eq!(candidate.candidate.parent_moment_id, root);
    assert_eq!(world.second_selection.parent_moment_id, root);
    assert_eq!(world.result.parent_moment_id, root);
    let roles = crate::suite::world::observed(campaign_core::project_frontier_roles(&world.state))?;
    let children = &world.result.children;
    assert_eq!(
        sorted(objects(&roles.unfolding)),
        sorted(children.iter().map(|child| child.discovery_id).collect())
    );
    assert_eq!(
        sorted(objects(&roles.reservations)),
        sorted(children.iter().map(|child| child.reservation_id).collect())
    );
    assert_eq!(objects(&roles.plans), vec![world.candidate_id]);
    assert_eq!(objects(&roles.admissions), vec![world.candidate_id]);
    assert_eq!(
        objects(&roles.schedules),
        vec![world.second_selection.selection_id]
    );
    assert_eq!(objects(&roles.commitments), vec![world.result.result_id]);
    assert_eq!(
        world.result.selection_id,
        world.first_selection.selection_id
    );
    assert!(!objects(&roles.schedules).contains(&world.first_selection.selection_id));
    assert_distinct_sections(&roles);
    assert_eq!(
        roles.role_projection_id.kind,
        campaign_core::IdKind::RoleProjection
    );
    crate::suite::world::observed(campaign_core::check_no_role_collapse(
        &campaign_core::RoleCollapseFixture {
            candidate_record_id: world.candidate_id,
            queued_work_id: world.second_selection.selection_id,
            computed_result_id: world.result.result_id,
        },
    ))?;
    Ok(())
}

#[test]
// r[verify campaign.fd.roles]
fn the_role_projection_identity_is_deterministic_and_state_sensitive() -> Result<(), String> {
    let first = crate::suite::world::populated()?;
    let second = crate::suite::world::populated()?;
    let left = crate::suite::world::observed(campaign_core::project_frontier_roles(&first.state))?;
    let right =
        crate::suite::world::observed(campaign_core::project_frontier_roles(&second.state))?;
    assert_eq!(left, right);
    assert_eq!(
        crate::suite::world::observed(campaign_core::derive_role_projection_id(&left))?,
        left.role_projection_id
    );
    let settled =
        crate::suite::world::observed(campaign_core::project_frontier_roles(&first.settled_state))?;
    assert!(settled.schedules.is_empty());
    assert_ne!(settled.role_projection_id, left.role_projection_id);
    Ok(())
}

#[test]
// r[verify campaign.fd.roles.collapsed]
fn a_collapsed_record_is_reported_as_a_role_defect() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let candidate = world.candidate_id;
    let selection = world.second_selection.selection_id;
    let result = world.result.result_id;
    let relabel = |id: campaign_core::Id, kind: campaign_core::IdKind| campaign_core::Id {
        kind,
        digest: id.digest,
    };
    let collapsed = [
        (candidate, candidate, candidate),
        (
            candidate,
            relabel(candidate, campaign_core::IdKind::Selection),
            result,
        ),
        (
            candidate,
            selection,
            relabel(candidate, campaign_core::IdKind::Result),
        ),
        (
            candidate,
            selection,
            relabel(selection, campaign_core::IdKind::Result),
        ),
    ];
    for (candidate_record_id, queued_work_id, computed_result_id) in collapsed {
        assert_eq!(
            crate::suite::world::denial(campaign_core::check_no_role_collapse(
                &campaign_core::RoleCollapseFixture {
                    candidate_record_id,
                    queued_work_id,
                    computed_result_id,
                }
            ))?,
            campaign_core::CampaignError::RoleCollapse
        );
    }
    assert_eq!(
        crate::suite::world::denial(campaign_core::check_no_role_collapse(
            &campaign_core::RoleCollapseFixture {
                candidate_record_id: selection,
                queued_work_id: candidate,
                computed_result_id: result,
            }
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}
