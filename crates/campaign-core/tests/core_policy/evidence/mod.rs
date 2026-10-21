#[test]
fn pruning_overclaims_and_outstanding_targets_fail() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"prune",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (_events, state) = crate::suite::selection::publish(events, selection)?;
    let request = campaign_core::PruneRequest {
        prune_plan_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        candidates: vec![candidate_id],
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"prune"),
        retention_profile_id: None,
        rank_order: Vec::new(),
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    };
    assert_eq!(
        (campaign_core::prune_frontier(&state, request.clone()))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::InvalidPruning
    );
    let mut overclaim = request;
    overclaim.claims_safe_deletion = true;
    assert_eq!(
        (campaign_core::prune_frontier(&state, overclaim))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::InvalidPruning
    );
    Ok(())
}
#[test]
// r[verify campaign.publication_fence]
fn publication_controls_execution() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"publish",
        crate::suite::constants::COST_ONE,
    )?;
    let (_events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let evidence = campaign_core::PublicationEvidence {
        event_id: selection.publication_plan.event_id,
        selection_id: selection.selection_id,
        branch_before: campaign_core::BranchFact {
            control_branch_id: selection.publication_plan.control_branch_id,
            head_id: selection.publication_plan.expected_head_id,
            generation: selection.publication_plan.expected_generation,
        },
        branch_after: campaign_core::BranchFact {
            control_branch_id: crate::suite::admission::xid(
                campaign_core::IdKind::ControlBranch,
                b"control-branch-after",
            ),
            head_id: crate::suite::admission::xid(campaign_core::IdKind::Head, b"head-after"),
            generation: (selection
                .publication_plan
                .expected_generation
                .checked_add(crate::suite::constants::COST_ONE))
            .ok_or_else(|| String::from("fixture arithmetic must stay in range"))?,
        },
        durable_event: false,
        durable_branch_update: true,
    };
    assert!(
        !(campaign_core::can_execute_published_selection(&selection, &evidence))
            .map_err(|error| format!("{error:?}"))?
    );
    let mut stale_generation = evidence.clone();
    stale_generation.branch_after.generation = selection.publication_plan.expected_generation;
    assert_eq!(
        (campaign_core::can_execute_published_selection(&selection, &stale_generation))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::InvalidPublication
    );
    let mut durable = evidence;
    durable.durable_event = true;
    assert!(
        (campaign_core::can_execute_published_selection(&selection, &durable))
            .map_err(|error| format!("{error:?}"))?
    );
    Ok(())
}
#[test]
// r[verify campaign.fd.verification]
fn frontier_discipline_negative_boundaries_fail() -> Result<(), String> {
    let (_events, state) = crate::suite::admission::initial_state()?;
    let roles =
        (campaign_core::project_frontier_roles(&state)).map_err(|error| format!("{error:?}"))?;
    assert!(roles.commitments.is_empty());
    let collapsed = campaign_core::RoleCollapseFixture {
        candidate_record_id: crate::suite::admission::xid(
            campaign_core::IdKind::Candidate,
            b"same",
        ),
        queued_work_id: campaign_core::Id {
            kind: campaign_core::IdKind::Selection,
            digest: crate::suite::admission::xid(campaign_core::IdKind::Candidate, b"same").digest,
        },
        computed_result_id: crate::suite::admission::xid(campaign_core::IdKind::Result, b"result"),
    };
    assert_eq!(
        (campaign_core::check_no_role_collapse(&collapsed))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::RoleCollapse
    );
    assert_eq!(
        (campaign_core::validate_dynamics(&campaign_core::FrontierDynamics::MonotoneRelaxation {
            relaxation_id: crate::suite::admission::xid(
                campaign_core::IdKind::Relaxation,
                b"relax"
            ),
            improved: false
        }))
        .err()
        .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::NonImprovingRelaxation
    );
    assert_eq!(
        (campaign_core::select_production_policy(
            &campaign_core::PolicyImplementation::ReferenceEvidence {
                reference_id: crate::suite::admission::xid(
                    campaign_core::IdKind::Policy,
                    b"reference"
                )
            }
        ))
        .err()
        .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    Ok(())
}
