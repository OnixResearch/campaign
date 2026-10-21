//! Discovery, monotone relaxation, and invalidation stay distinct guidance dynamics.

const SECOND_GENERATION: u64 = 2;
const THIRD_GENERATION: u64 = 3;

#[test]
// r[verify campaign.fd.dynamics.relax]
// r[verify campaign.guidance.non_improving]
fn a_non_improving_relaxation_schedules_nothing_and_leaves_state_unchanged() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let before = crate::suite::selection::plan(&state)?;
    let worse = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![crate::suite::constants::RANK_LOW],
    );
    assert_eq!(
        campaign_core::admit_guidance(&state, worse.clone()),
        Err(campaign_core::CampaignError::NonImprovingRelaxation)
    );
    crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    assert!(state.outstanding.is_empty());
    let after = crate::suite::selection::plan(&state)?;
    assert_eq!(after, before);
    assert_eq!(after.guidance_id, previous.guidance_id);
    let claimed = campaign_core::GuidanceDynamics::StrictImprovement {
        previous_guidance_id: previous.guidance_id,
    };
    let equal = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![crate::suite::constants::RANK_HIGH],
    );
    for (record, expected) in [
        (worse, campaign_core::CampaignError::NonImprovingRelaxation),
        (equal, campaign_core::CampaignError::StaleGuidance),
    ] {
        let forged = crate::suite::ingress::support::forged_guidance(
            &state,
            crate::suite::ingress::support::text(record.derive())?,
            claimed.clone(),
        )?;
        assert_eq!(
            crate::suite::ingress::support::replay_with(&events, forged),
            Err(expected)
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    Ok(())
}

#[test]
// r[verify campaign.fd.dynamics.relax]
// r[verify campaign.guidance.non_improving]
fn a_forged_improvement_over_worse_ranks_is_a_non_improving_relaxation() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let worse = crate::suite::ingress::support::text(
        crate::suite::ingress::support::guidance(
            &state,
            candidate_id,
            SECOND_GENERATION,
            vec![crate::suite::constants::RANK_LOW],
        )
        .derive(),
    )?;
    let forged = crate::suite::ingress::support::forged_guidance(
        &state,
        worse,
        campaign_core::GuidanceDynamics::StrictImprovement {
            previous_guidance_id: previous.guidance_id,
        },
    )?;
    assert_eq!(
        crate::suite::ingress::support::replay_with(&events, forged),
        Err(campaign_core::CampaignError::NonImprovingRelaxation)
    );
    crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    assert_ne!(previous.ranks, vec![crate::suite::constants::RANK_LOW]);
    Ok(())
}

#[test]
// r[verify campaign.fd.dynamics]
fn each_guidance_dynamic_projects_onto_a_distinct_frontier_dynamic() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_LOW)?;
    let first = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let discovery = campaign_core::GuidanceDynamics::Discovery;
    assert!(matches!(
        crate::suite::ingress::support::text(discovery.frontier_dynamics(None, &first))?,
        Some(campaign_core::FrontierDynamics::Discovery { .. })
    ));
    let improved = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![crate::suite::constants::RANK_HIGH],
    );
    let event =
        crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, improved))?;
    let relaxation = crate::suite::ingress::support::guidance_of(&event)?.clone();
    let projected = crate::suite::ingress::support::text(
        relaxation
            .dynamics
            .frontier_dynamics(Some(&first), &relaxation.guidance),
    )?;
    assert!(matches!(
        projected,
        Some(campaign_core::FrontierDynamics::MonotoneRelaxation { improved: true, .. })
    ));
    let (events, state) = crate::suite::ingress::support::accept(events, event)?;
    let revision = crate::suite::ingress::support::text(campaign_core::revise_guidance(
        &state,
        campaign_core::GuidanceRevisionRequest {
            candidate_id,
            successor: crate::suite::ingress::support::guidance(
                &state,
                candidate_id,
                THIRD_GENERATION,
                vec![crate::suite::constants::RANK_LOW],
            ),
            invalidated_guidance_id: relaxation.guidance.guidance_id,
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"revise"),
        },
    ))?;
    let revised = crate::suite::ingress::support::guidance_of(&revision)?.clone();
    let projected = crate::suite::ingress::support::text(
        revised
            .dynamics
            .frontier_dynamics(Some(&relaxation.guidance), &revised.guidance),
    )?;
    let Some(campaign_core::FrontierDynamics::Invalidation {
        invalidation_id,
        cleared_dependency_id,
    }) = projected
    else {
        return Err(format!(
            "revision must be an invalidation, got {projected:?}"
        ));
    };
    assert_eq!(cleared_dependency_id, relaxation.guidance.guidance_id);
    assert_eq!(invalidation_id.kind, campaign_core::IdKind::Invalidation);
    assert!(matches!(
        revised.dynamics,
        campaign_core::GuidanceDynamics::Revision { .. }
    ));
    let (_events, next) = crate::suite::ingress::support::accept(events, revision)?;
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?,
        revised.guidance
    );
    Ok(())
}

#[test]
fn coalescing_adds_no_record_and_no_frontier_dynamic() -> Result<(), String> {
    let (_events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_LOW)?;
    let first = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let coalesced = campaign_core::GuidanceDynamics::CoalescedDuplicate {
        existing_guidance_id: first.guidance_id,
    };
    assert_eq!(
        crate::suite::ingress::support::text(coalesced.frontier_dynamics(Some(&first), &first))?,
        None
    );
    assert!(!coalesced.adds_record());
    Ok(())
}
