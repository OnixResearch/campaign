//! `campaign.receipts` controls: determinism, crossed inputs, and non-claims.

fn crossed(
    planned: &crate::suite::world::Planned,
    plan: &campaign_core::DecisionPlan,
    request: &campaign_core::SelectionRequest,
    binding: &campaign_core::ProjectionBinding,
) -> Result<campaign_core::CampaignError, String> {
    crate::suite::world::denial(campaign_core::project_receipt(
        &planned.state,
        plan,
        request,
        binding,
    ))
}

#[test]
// r[verify campaign.receipts.deterministic]
fn equal_decision_inputs_are_stable() -> Result<(), String> {
    let first = crate::suite::world::selecting()?;
    let second = crate::suite::world::selecting()?;
    let left = crate::suite::world::observed(crate::suite::world::project(&first))?;
    let right = crate::suite::world::observed(crate::suite::world::project(&second))?;
    assert_eq!(left, right);
    assert_eq!(left.receipt_id.kind, campaign_core::IdKind::Receipt);
    let campaign_core::NextDecision::Select(selection) = &first.plan.decision else {
        return Err(String::from("fixture must select"));
    };
    assert_eq!(left.decision_id, selection.selection_id);
    assert_eq!(left.guidance_record_id, Some(selection.guidance_id));
    assert_eq!(left.entropy_ticket_id, None);
    assert_eq!(
        left.cost_reservation_id,
        Some(crate::suite::world::observed(
            campaign_core::derive_cost_reservation_id(selection)
        )?)
    );
    assert_eq!(
        left.role_projection_id,
        crate::suite::world::observed(campaign_core::project_frontier_roles(&first.state))?
            .role_projection_id
    );
    assert_eq!(left.projection_id, first.binding.projection_id);
    assert_eq!(left.selection_profile_id, first.request.profile.profile_id);
    assert_eq!(left.admission_outcomes_id, None);
    let mut other_request = first.request.clone();
    other_request.request_id =
        crate::suite::admission::xid(campaign_core::IdKind::Request, b"other-receipt-request");
    let changed = crate::suite::world::observed(campaign_core::project_receipt(
        &first.state,
        &first.plan,
        &other_request,
        &first.binding,
    ))?;
    assert_ne!(changed.receipt_id, left.receipt_id);
    Ok(())
}

#[test]
// r[verify campaign.receipts.deterministic]
fn wait_and_stop_decisions_are_stable() -> Result<(), String> {
    let (events, empty) = crate::suite::admission::initial_state()?;
    let (_events, unguided, _candidate) = crate::suite::candidate::admit(
        events,
        &empty,
        b"unguided",
        crate::suite::constants::COST_ONE,
    )?;
    let waiting = crate::suite::world::planned(unguided)?;
    assert_eq!(
        waiting.plan.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    let stopping = crate::suite::world::planned(empty)?;
    assert!(matches!(
        stopping.plan.decision,
        campaign_core::NextDecision::Stop(_)
    ));
    for fixture in [&waiting, &stopping] {
        let left = crate::suite::world::observed(crate::suite::world::project(fixture))?;
        let right = crate::suite::world::observed(crate::suite::world::project(fixture))?;
        assert_eq!(left, right);
        assert_eq!(left.guidance_record_id, None);
        assert_eq!(left.cost_reservation_id, None);
    }
    Ok(())
}

#[test]
// r[verify campaign.receipts.crossed]
fn a_decision_from_another_campaign_or_state_is_crossed() -> Result<(), String> {
    let fixture = crate::suite::world::selecting()?;
    let binding = fixture.binding.clone();
    let mut other_input = crate::suite::admission::default_input()?;
    other_input.budget_profile_id =
        crate::suite::admission::xid(campaign_core::IdKind::BudgetProfile, b"other-budget");
    let (other_events, other_state) = crate::suite::admission::admitted(other_input)?;
    let (other_events, other_state, other_candidate) = crate::suite::candidate::admit(
        other_events,
        &other_state,
        b"populated",
        crate::suite::constants::COST_ONE,
    )?;
    let (_other_events, other_state, _guidance) = crate::suite::candidate::guide(
        other_events,
        &other_state,
        other_candidate,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let other_campaign = crate::suite::world::planned(other_state)?;
    assert_ne!(other_campaign.plan.campaign_id, fixture.plan.campaign_id);
    assert_eq!(
        crossed(&fixture, &other_campaign.plan, &fixture.request, &binding)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut foreign_selection = fixture.plan.clone();
    if let campaign_core::NextDecision::Select(selection) = &mut foreign_selection.decision {
        selection.campaign_id = other_campaign.plan.campaign_id;
    }
    assert_eq!(
        crossed(&fixture, &foreign_selection, &fixture.request, &binding)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let world = crate::suite::world::populated()?;
    let earlier = crate::suite::world::planned(world.state)?;
    assert_eq!(
        crossed(&fixture, &earlier.plan, &fixture.request, &binding)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut stale_selection = fixture.plan.clone();
    if let campaign_core::NextDecision::Select(selection) = &mut stale_selection.decision {
        selection.state_before_id = earlier.state.state_id;
    }
    assert_eq!(
        crossed(&fixture, &stale_selection, &fixture.request, &binding)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}

#[test]
// r[verify campaign.receipts.crossed]
fn a_crossed_request_policy_or_history_is_rejected() -> Result<(), String> {
    let fixture = crate::suite::world::selecting()?;
    let binding = fixture.binding.clone();
    let mut other_state_request = fixture.request.clone();
    other_state_request.state_id =
        crate::suite::admission::xid(campaign_core::IdKind::State, b"other-state");
    let mut other_campaign_request = fixture.request.clone();
    other_campaign_request.campaign_id =
        crate::suite::admission::xid(campaign_core::IdKind::Campaign, b"other-campaign");
    for request in [&other_state_request, &other_campaign_request] {
        assert_eq!(
            crossed(&fixture, &fixture.plan, request, &binding)?,
            campaign_core::CampaignError::CrossedIdentity
        );
    }
    let mut other_policy = fixture.request.profile.clone();
    other_policy.admission_policy_id =
        crate::suite::admission::xid(campaign_core::IdKind::AdmissionPolicy, b"other-admission");
    let mut policy_request = fixture.request.clone();
    policy_request.profile = crate::suite::world::observed(other_policy.derive())?;
    assert_eq!(
        crossed(&fixture, &fixture.plan, &policy_request, &binding)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut other_head = binding.clone();
    other_head.head_id = crate::suite::admission::xid(campaign_core::IdKind::Head, b"other-head");
    let other_head = crate::suite::world::observed(other_head.derive())?;
    let mut other_adapter = binding.clone();
    other_adapter.adapter_id =
        crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"other-adapter");
    let other_adapter = crate::suite::world::observed(other_adapter.derive())?;
    for history in [&other_head, &other_adapter] {
        assert_eq!(
            crossed(&fixture, &fixture.plan, &fixture.request, history)?,
            campaign_core::CampaignError::CrossedIdentity
        );
    }
    let mut forged_binding = binding;
    forged_binding.generation = crate::suite::constants::GENERATION_NEXT;
    assert_eq!(
        crossed(&fixture, &fixture.plan, &fixture.request, &forged_binding)?,
        campaign_core::CampaignError::StaleProjection
    );
    let mut stale = fixture.state.clone();
    stale.selection_count = crate::suite::constants::BOUND_MANY;
    assert_eq!(
        crate::suite::world::denial(campaign_core::project_receipt(
            &stale,
            &fixture.plan,
            &fixture.request,
            &fixture.binding,
        ))?,
        campaign_core::CampaignError::StaleState
    );
    Ok(())
}

#[test]
fn an_exploratory_selection_binds_its_entropy_ticket() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let profile = crate::suite::schedules::always_explore(&frontier.state)?;
    let ticket = crate::suite::schedules::ticket(
        &frontier.state,
        &profile,
        crate::suite::schedules::EXPLORE_SECOND,
    )?;
    let binding = &crate::suite::admission::current_projection(&frontier.state)?;
    let (plan, request) = crate::suite::schedules::planned_on(
        &frontier.state,
        profile.clone(),
        Some(ticket.clone()),
        binding,
    )?;
    let projected = crate::suite::world::observed(campaign_core::project_receipt(
        &frontier.state,
        &plan,
        &request,
        binding,
    ))?;
    assert_eq!(projected.entropy_ticket_id, Some(ticket.entropy_id));
    let other_ticket = crate::suite::schedules::ticket(
        &frontier.state,
        &profile,
        crate::suite::schedules::EXPLORE_FIRST,
    )?;
    let mut crossed_ticket = request;
    crossed_ticket.entropy = Some(other_ticket);
    assert_eq!(
        crate::suite::world::denial(campaign_core::project_receipt(
            &frontier.state,
            &plan,
            &crossed_ticket,
            binding,
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}
