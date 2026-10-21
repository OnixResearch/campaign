//! Exact candidate selection (`campaign.selection`).

const RANK_TOP: i64 = 20;
const RANK_IMPROVED: i64 = 30;
const SECOND_GENERATION: u64 = 2;
const NEXT_GENERATION: u64 = 8;

fn with_status(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    status: &campaign_core::CandidateStatus,
) -> Result<campaign_core::FrontierState, String> {
    crate::suite::planning_support::rebuilt(state.clone(), |forged| {
        for candidate in &mut forged.candidates {
            if candidate.candidate.candidate_id == candidate_id {
                candidate.status = status.clone();
            }
        }
    })
}

#[test]
// r[verify campaign.selection]
// r[verify campaign.selection.valid]
fn the_best_eligible_candidate_yields_one_exactly_bound_selection() -> Result<(), String> {
    let (_events, state, top, _bottom) = crate::suite::planning_support::two_guided()?;
    let profile = crate::suite::selection::ranked_profile()?;
    let plan = crate::suite::planning_support::ranked(&state)?;
    assert_eq!(plan, crate::suite::planning_support::ranked(&state)?);
    let selection = crate::suite::planning_support::selected(&plan)?;
    let candidate = crate::suite::planning_support::observed(state.candidate(top))?;
    let guidance = candidate
        .latest_guidance
        .as_ref()
        .ok_or_else(|| String::from("top must be guided"))?;
    assert_eq!(selection.candidate_id, top);
    assert_eq!(
        selection.ordinal,
        crate::suite::planning_support::FIRST_ORDINAL
    );
    assert_eq!(selection.campaign_id, state.admission.campaign_id);
    assert_eq!(selection.state_before_id, state.state_id);
    assert_eq!(
        selection.parent_moment_id,
        candidate.candidate.parent_moment_id
    );
    assert_eq!(selection.guidance_id, guidance.guidance_id);
    assert_eq!(selection.profile_id, profile.profile_id);
    assert_eq!(selection.entropy_id, None);
    assert_eq!(selection.reserved_costs, candidate.candidate.declared_costs);
    assert_eq!(
        selection.operation_id.kind,
        campaign_core::IdKind::Operation
    );
    assert_eq!(
        selection.publication_plan.expected_generation,
        state.admission.projection.generation
    );
    assert_eq!(
        selection.publication_plan.expected_head_id,
        state.admission.projection.head_id
    );
    Ok(())
}

#[test]
// r[verify campaign.selection.invalid]
fn pruned_or_retired_candidates_are_never_selected() -> Result<(), String> {
    let (_events, state, top, bottom) = crate::suite::planning_support::two_guided()?;
    let statuses = [
        campaign_core::CandidateStatus::Pruned {
            prune_plan_id: crate::suite::admission::xid(campaign_core::IdKind::PrunePlan, b"prune"),
        },
        campaign_core::CandidateStatus::Retired {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"retire"),
        },
    ];
    for status in &statuses {
        let without_top = with_status(&state, top, status)?;
        assert_eq!(
            crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(
                &without_top
            )?)?
            .candidate_id,
            bottom
        );
        let neither = with_status(&without_top, bottom, status)?;
        let stop = crate::suite::planning_support::stopped(
            &crate::suite::planning_support::ranked(&neither)?,
        )?;
        assert_eq!(stop.reason, campaign_core::StopReason::FrontierExhausted);
    }
    Ok(())
}

#[test]
fn replayed_selection_of_an_absent_candidate_fails() -> Result<(), String> {
    let (events, state, _top, _bottom) = crate::suite::planning_support::two_guided()?;
    let selection =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    let absent = crate::suite::planning_support::forged_selection(&selection, |decision| {
        decision.candidate_id =
            crate::suite::admission::xid(campaign_core::IdKind::Candidate, b"absent");
    })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, absent),
        Err(campaign_core::CampaignError::MissingCandidate)
    );
    Ok(())
}

#[test]
// r[verify campaign.selection.invalid]
fn replayed_selection_over_budget_or_outstanding_bound_fails() -> Result<(), String> {
    let (events, state) = crate::suite::planning_support::admitted_with(|input| {
        input.resource_budgets[0].amount = crate::suite::constants::BUDGET_SMALL;
    })?;
    let (events, state, _only) =
        crate::suite::planning_support::guided(events, &state, b"only", RANK_TOP)?;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let over_budget = crate::suite::planning_support::forged_selection(&first, |decision| {
        decision.state_before_id = state.state_id;
        decision.ordinal = crate::suite::planning_support::SECOND_ORDINAL;
    })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, over_budget),
        Err(campaign_core::CampaignError::BudgetExhausted)
    );
    let (events, state) = crate::suite::planning_support::admitted_with(|input| {
        input.bounds.max_outstanding = crate::suite::constants::BOUND_OUTSTANDING_ONE;
    })?;
    let (events, state, _only) =
        crate::suite::planning_support::guided(events, &state, b"only", RANK_TOP)?;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let over_outstanding = crate::suite::planning_support::forged_selection(&first, |decision| {
        decision.state_before_id = state.state_id;
        decision.ordinal = crate::suite::planning_support::SECOND_ORDINAL;
    })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, over_outstanding),
        Err(campaign_core::CampaignError::OutstandingLimitReached)
    );
    Ok(())
}

#[test]
// r[verify campaign.selection.invalid]
fn stale_state_or_guidance_emits_no_selection() -> Result<(), String> {
    let (events, state, top, _bottom) = crate::suite::planning_support::two_guided()?;
    let stale_plan =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    let (events, current, _improved) = crate::suite::planning_support::guide_ranks(
        events,
        &state,
        top,
        &[RANK_IMPROVED],
        SECOND_GENERATION,
    )?;
    let mut stale_request = crate::suite::selection::request(
        &current,
        crate::suite::selection::ranked_profile()?,
        None,
    );
    stale_request.state_id = state.state_id;
    assert_eq!(
        campaign_core::plan_next(&current, stale_request, &current.admission.projection),
        Err(campaign_core::CampaignError::StaleState)
    );
    assert_eq!(
        crate::suite::planning_support::replay_with(
            &events,
            crate::suite::planning_support::forged_selection(&stale_plan, |_| {})?
        ),
        Err(campaign_core::CampaignError::StaleState)
    );
    let stale_guidance =
        crate::suite::planning_support::forged_selection(&stale_plan, |decision| {
            decision.state_before_id = current.state_id;
        })?;
    assert_eq!(
        crate::suite::planning_support::replay_with(&events, stale_guidance),
        Err(campaign_core::CampaignError::StaleGuidance)
    );
    let fresh = crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(
        &current,
    )?)?;
    assert_ne!(fresh.guidance_id, stale_plan.guidance_id);
    Ok(())
}

#[test]
// r[verify campaign.selection.reuse]
fn a_settled_candidate_is_selected_again_with_a_successor_ordinal() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, only) =
        crate::suite::planning_support::guided(events, &state, b"only", RANK_TOP)?;
    let (events, state, first) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let (events, state) = crate::suite::planning_support::settle(
        events,
        &state,
        &first,
        (b"reuse-child", b"reuse-payload"),
        Vec::new(),
    )?;
    let (_events, _state, second) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    assert_eq!(second.candidate_id, only);
    assert_eq!(second.guidance_id, first.guidance_id);
    assert_eq!(
        second.ordinal,
        crate::suite::planning_support::SECOND_ORDINAL
    );
    assert_ne!(second.selection_id, first.selection_id);
    assert_ne!(second.operation_id, first.operation_id);
    Ok(())
}

#[test]
// r[verify campaign.selection.non_claim]
fn a_selection_decision_claims_no_execution_or_result() -> Result<(), String> {
    let (events, state, _top, _bottom) = crate::suite::planning_support::two_guided()?;
    let selection =
        crate::suite::planning_support::selected(&crate::suite::planning_support::ranked(&state)?)?;
    let unchanged = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(unchanged, state);
    assert!(unchanged.outstanding.is_empty());
    let (_events, published) = crate::suite::selection::publish(events, selection.clone())?;
    assert!(
        published
            .outstanding
            .iter()
            .any(|outstanding| outstanding.selection_id == selection.selection_id)
    );
    assert!(published.accepted_results.is_empty());
    assert!(published.settled.is_empty());
    let plan = &selection.publication_plan;
    let before = campaign_core::BranchFact {
        control_branch_id: plan.control_branch_id,
        head_id: plan.expected_head_id,
        generation: plan.expected_generation,
    };
    let evidence = campaign_core::PublicationEvidence {
        event_id: plan.event_id,
        selection_id: selection.selection_id,
        branch_before: before,
        branch_after: campaign_core::BranchFact {
            head_id: crate::suite::admission::xid(campaign_core::IdKind::Head, b"after"),
            generation: NEXT_GENERATION,
            ..before
        },
        durable_event: false,
        durable_branch_update: false,
    };
    assert_eq!(
        campaign_core::can_execute_published_selection(&selection, &evidence),
        Ok(false)
    );
    Ok(())
}
