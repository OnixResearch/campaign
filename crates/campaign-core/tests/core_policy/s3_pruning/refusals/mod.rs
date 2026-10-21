//! Rejected prune requests and retention plans leave the frontier identity unchanged.

fn assert_refused(
    frontier: &crate::suite::s3_pruning::Frontier,
    request: campaign_core::PruneRequest,
    expected: &campaign_core::CampaignError,
) -> Result<(), String> {
    let refused = crate::suite::s3_fixture::denial(campaign_core::prune_frontier(
        &frontier.state,
        request.clone(),
    ))?;
    assert_eq!(&refused, expected);
    let replayed = crate::suite::s3_fixture::replay_denial(
        &frontier.events,
        crate::suite::s3_pruning::forged_prune(&frontier.state, request)?,
    )?;
    assert_eq!(&replayed, expected);
    crate::suite::s3_fixture::assert_unchanged(&frontier.events, &frontier.state)
}

#[test]
// r[verify campaign.pruning.unchanged]
fn a_profile_request_must_name_exactly_its_planned_targets() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::NARROW_BATCH)?;
    let state = &frontier.state;
    let plan = campaign_core::plan_pruning(
        state,
        crate::suite::s3_pruning::plan_request(
            state,
            vec![crate::suite::s3_pruning::FIRST_DIMENSION],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or_else(|| String::from("expected a prune plan"))?;
    let mut chosen = plan;
    chosen.candidates = vec![frontier.middle, frontier.lowest];
    assert_refused(
        &frontier,
        chosen,
        &campaign_core::CampaignError::InvalidPruning,
    )?;
    let mut undeclared_order = crate::suite::s3_pruning::explicit(state, vec![frontier.middle]);
    undeclared_order.rank_order = vec![crate::suite::s3_pruning::FIRST_DIMENSION];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::prune_frontier(state, undeclared_order))?,
        campaign_core::CampaignError::InvalidConfig
    );
    let event = campaign_core::prune_frontier(
        state,
        crate::suite::s3_pruning::explicit(state, vec![frontier.middle]),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, pruned) = crate::suite::s3_fixture::append(frontier.events.clone(), event)?;
    assert!(matches!(
        pruned
            .candidate(frontier.middle)
            .map_err(crate::suite::s3_fixture::err)?
            .status,
        campaign_core::CandidateStatus::Pruned { .. }
    ));
    Ok(())
}

/// A profile request (and its replayed event) whose rank order names a dimension beyond the
/// fixed rank shape is refused as configuration, even when it lists the targets of the plan.
#[test]
// r[verify campaign.guidance.declared_order]
fn a_profile_request_ordered_beyond_the_rank_shape_is_refused_as_config() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::NARROW_BATCH)?;
    let plan = campaign_core::plan_pruning(
        &frontier.state,
        crate::suite::s3_pruning::plan_request(
            &frontier.state,
            vec![crate::suite::s3_pruning::FIRST_DIMENSION],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or_else(|| String::from("expected a prune plan"))?;
    assert_refused(
        &frontier,
        campaign_core::PruneRequest {
            rank_order: vec![crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION],
            ..plan.clone()
        },
        &campaign_core::CampaignError::InvalidConfig,
    )?;
    let event = campaign_core::prune_frontier(&frontier.state, plan.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidatesPruned(prune) = &event else {
        return Err(String::from("expected a prune event"));
    };
    assert_eq!(prune.pruned_candidates, plan.candidates);
    let (_events, pruned) = crate::suite::s3_fixture::append(frontier.events.clone(), event)?;
    assert_ne!(pruned.state_id, frontier.state.state_id);
    Ok(())
}

#[test]
// r[verify campaign.pruning.invalid]
fn history_rewrite_artifact_deletion_and_safe_deletion_claims_are_denied() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let base = crate::suite::s3_pruning::explicit(&frontier.state, vec![frontier.middle]);
    let claims = [
        campaign_core::PruneRequest {
            claims_history_rewrite: true,
            ..base.clone()
        },
        campaign_core::PruneRequest {
            claims_artifact_deletion: true,
            ..base.clone()
        },
        campaign_core::PruneRequest {
            claims_safe_deletion: true,
            ..base.clone()
        },
    ];
    for request in claims {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::prune_frontier(
                &frontier.state,
                request
            ))?,
            campaign_core::CampaignError::InvalidPruning
        );
        crate::suite::s3_fixture::assert_unchanged(&frontier.events, &frontier.state)?;
    }
    let event = campaign_core::prune_frontier(&frontier.state, base)
        .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidatesPruned(prune) = event else {
        return Err(String::from("expected a prune event"));
    };
    assert_eq!(prune.pruned_candidates, vec![frontier.middle]);
    Ok(())
}

#[test]
// r[verify campaign.pruning.invalid]
fn outstanding_unknown_empty_and_non_retained_targets_are_denied() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let state = &frontier.state;
    assert!(state.has_outstanding_for_candidate(frontier.top));
    assert_refused(
        &frontier,
        crate::suite::s3_pruning::explicit(state, vec![frontier.top]),
        &campaign_core::CampaignError::InvalidPruning,
    )?;
    let unknown = crate::suite::admission::xid(campaign_core::IdKind::Candidate, b"unknown");
    assert_refused(
        &frontier,
        crate::suite::s3_pruning::explicit(state, vec![unknown]),
        &campaign_core::CampaignError::MissingCandidate,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::prune_frontier(
            state,
            crate::suite::s3_pruning::explicit(state, Vec::new()),
        ))?,
        campaign_core::CampaignError::UnchangedState
    );
    let retired = campaign_core::retire_candidate(
        state,
        crate::suite::s3_fixture::retirement(
            state,
            frontier.kept,
            campaign_core::RetirementOrigin::Policy,
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(frontier.events.clone(), retired)?;
    let after_retirement = crate::suite::s3_pruning::Frontier {
        events,
        state,
        ..frontier
    };
    let request =
        crate::suite::s3_pruning::explicit(&after_retirement.state, vec![after_retirement.kept]);
    assert_refused(
        &after_retirement,
        request,
        &campaign_core::CampaignError::InvalidPruning,
    )
}

#[test]
// r[verify campaign.pruning.unchanged]
fn stale_or_undeclared_retention_plans_are_denied() -> Result<(), String> {
    let (_events, admitted) = crate::suite::admission::initial_state()?;
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let state = &frontier.state;
    let request = |order: Vec<u32>| crate::suite::s3_pruning::plan_request(state, order);
    let first = vec![crate::suite::s3_pruning::FIRST_DIMENSION];
    let cases = [
        (
            campaign_core::RetentionPlanRequest {
                state_id: admitted.state_id,
                ..request(first.clone())
            },
            campaign_core::CampaignError::StaleState,
        ),
        (
            campaign_core::RetentionPlanRequest {
                retention_profile_id: crate::suite::admission::xid(
                    campaign_core::IdKind::Policy,
                    b"not-retention",
                ),
                ..request(first.clone())
            },
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (
            request(Vec::new()),
            campaign_core::CampaignError::InvalidConfig,
        ),
        (
            request(vec![
                crate::suite::s3_pruning::FIRST_DIMENSION,
                crate::suite::s3_pruning::FIRST_DIMENSION,
            ]),
            campaign_core::CampaignError::InvalidConfig,
        ),
        (
            request(vec![crate::suite::s3_pruning::OUT_OF_BOUND_DIMENSION]),
            campaign_core::CampaignError::InvalidConfig,
        ),
        (
            request(vec![crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION]),
            campaign_core::CampaignError::InvalidConfig,
        ),
        (
            request(vec![
                crate::suite::s3_pruning::FIRST_DIMENSION,
                crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION,
            ]),
            campaign_core::CampaignError::InvalidConfig,
        ),
    ];
    for (plan, expected) in cases {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::plan_pruning(state, plan))?,
            expected
        );
        crate::suite::s3_fixture::assert_unchanged(&frontier.events, state)?;
    }
    assert!(
        campaign_core::plan_pruning(state, request(first))
            .map_err(crate::suite::s3_fixture::err)?
            .is_some()
    );
    Ok(())
}
