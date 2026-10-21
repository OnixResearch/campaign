#[test]
fn selection_binds_expected_head() -> Result<(), String> {
    let (events, state) =
        crate::suite::admission::admitted_state(crate::suite::admission::Limits {
            resource_budget: crate::suite::constants::RESOURCE_BUDGET_TOTAL,
            max_findings: crate::suite::constants::BOUND_MANY,
        })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"head-bound-candidate",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state) = crate::suite::candidate::guide(events, &state, candidate_id)?;
    let selection = crate::suite::selection::plan(&state)?;
    let mut valid_history = events.clone();
    valid_history.push(crate::suite::selection::published(selection.clone())?);
    let verified =
        campaign_core::reconstruct(&valid_history).map_err(|error| format!("{error:?}"))?;
    assert_eq!(verified.admission.campaign_id, state.admission.campaign_id);
    let mut altered_selection = selection;
    altered_selection.publication_plan.expected_head_id =
        crate::suite::admission::tid(campaign_core::IdKind::Head, b"forged-expected-head");
    let mut altered_history = events;
    altered_history.push(crate::suite::selection::published(altered_selection)?);
    assert!(
        campaign_core::reconstruct(&altered_history).is_err(),
        "r[campaign.selection]: reconstructed selections must bind the exact expected head"
    );
    Ok(())
}
#[test]
fn selection_must_reserve_the_candidate_declared_costs() -> Result<(), String> {
    let (events, state) =
        crate::suite::admission::admitted_state(crate::suite::admission::Limits {
            resource_budget: crate::suite::constants::RESOURCE_BUDGET_PAIR,
            max_findings: crate::suite::constants::BOUND_MANY,
        })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"cost-bound-candidate",
        crate::suite::constants::COST_TWO,
    )?;
    let (events, state) = crate::suite::candidate::guide(events, &state, candidate_id)?;
    let selection = crate::suite::selection::plan(&state)?;
    assert_eq!(
        selection.reserved_costs[0].amount,
        crate::suite::constants::COST_TWO,
        "control fixture must reserve the declared candidate cost"
    );
    let mut valid_history = events.clone();
    valid_history.push(crate::suite::selection::published(selection.clone())?);
    (campaign_core::reconstruct(&valid_history)).map_err(|error| format!("{error:?}"))?;
    let mut altered_selection = selection;
    altered_selection.reserved_costs[0].amount = crate::suite::constants::COST_ONE;
    let altered_selection = (altered_selection.derive()).map_err(|error| format!("{error:?}"))?;
    let mut altered_history = events;
    altered_history.push(crate::suite::selection::published(altered_selection)?);
    assert!(
        campaign_core::reconstruct(&altered_history).is_err(),
        "r[campaign.budgets.valid]: reconstruction must reject cost reservations below the candidate declaration"
    );
    Ok(())
}
#[test]
fn public_frontier_state_fields_must_not_admit_unpublished_candidates() -> Result<(), String> {
    let (_events, state) =
        crate::suite::admission::admitted_state(crate::suite::admission::Limits {
            resource_budget: crate::suite::constants::RESOURCE_BUDGET_TOTAL,
            max_findings: crate::suite::constants::BOUND_MANY,
        })?;
    let decision = campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state)?,
        &state.admission.projection,
    )
    .map_err(|error| format!("{error:?}"))?
    .decision;
    let campaign_core::NextDecision::Stop(stop) = decision else {
        return Err(format!("expected empty frontier stop, got {decision:?}"));
    };
    assert!(matches!(
        stop.reason,
        campaign_core::StopReason::FrontierExhausted
    ));
    let mut forged_state = state.clone();
    let candidate = (crate::suite::candidate::record(
        &forged_state,
        b"unpublished-candidate",
        crate::suite::constants::COST_ONE,
    )
    .derive())
    .map_err(|error| format!("{error:?}"))?;
    let guidance = (crate::suite::candidate::guidance(
        &forged_state,
        candidate.candidate_id,
        forged_state.admission.guidance_profile_id,
    )
    .derive())
    .map_err(|error| format!("{error:?}"))?;
    forged_state.candidates.push(campaign_core::CandidateState {
        candidate,
        status: campaign_core::CandidateStatus::Retained,
        selection_count: 0,
        latest_guidance: Some(guidance),
    });
    assert_eq!(
        campaign_core::plan_next(
            &forged_state,
            crate::suite::selection::request(&forged_state)?,
            &forged_state.admission.projection,
        ),
        Err(campaign_core::CampaignError::StaleState),
        "r[campaign.cache.invalid]: public state mutation must not justify work through an unchanged state ID"
    );
    Ok(())
}
